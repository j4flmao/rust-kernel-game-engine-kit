//! Bounded two-pass UI layout: measure values, then assign rectangles.

use super::budget::{UiConfig, UiDiagnostics, UiError};
use super::components::{UiAlign, UiDirection, UiOverflow, UiPosition, UiRect};
use super::id::UiNodeId;
use super::tree::UiTree;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LayoutStats {
    pub visited: usize,
    pub dirty: usize,
    pub iterations: usize,
}

pub struct UiLayoutEngine {
    config: UiConfig,
    order: Vec<UiNodeId>,
    intrinsic: Vec<(UiNodeId, UiRect)>,
}

impl UiLayoutEngine {
    pub fn try_new(config: UiConfig) -> Result<Self, UiError> {
        let mut order = Vec::new();
        order
            .try_reserve_exact(config.max_nodes)
            .map_err(|_| UiError::AllocationFailed)?;
        let mut intrinsic = Vec::new();
        intrinsic
            .try_reserve_exact(config.max_nodes)
            .map_err(|_| UiError::AllocationFailed)?;
        Ok(Self {
            config,
            order,
            intrinsic,
        })
    }
    pub fn layout(
        &mut self,
        tree: &mut UiTree,
        viewport: UiRect,
        diagnostics: &mut UiDiagnostics,
    ) -> Result<LayoutStats, UiError> {
        if !viewport.sane() {
            return Err(UiError::InvalidValue);
        }
        if !self.config.dpi_scale.is_finite() || self.config.dpi_scale <= 0.0 {
            return Err(UiError::InvalidValue);
        }
        tree.preorder(&mut self.order)?;
        self.measure(tree)?;
        tree.set_rect(tree.root(), viewport)?;
        let mut stats = LayoutStats {
            visited: 0,
            dirty: 0,
            iterations: 1,
        };
        for node in self.order.iter().copied().skip(1) {
            let parent = tree.parent(node)?.ok_or(UiError::InvalidParent)?;
            let parent_rect = tree.rect(parent)?;
            let siblings = tree.children(parent)?;
            let index = siblings
                .iter()
                .position(|id| *id == node)
                .ok_or(UiError::InvalidNode)?;
            let content = tree.content(node)?.clone();
            if !content.sane() {
                return Err(UiError::InvalidValue);
            }
            let parent_content = tree.content(parent)?.clone();
            let inner = UiRect {
                x: parent_rect.x + parent_content.style.padding.left,
                y: parent_rect.y + parent_content.style.padding.top,
                width: (parent_rect.width - parent_content.style.padding.horizontal()).max(0.0),
                height: (parent_rect.height - parent_content.style.padding.vertical()).max(0.0),
            };
            let count = siblings.len().max(1) as f32;
            let gap_total = parent_content.style.gap * (count - 1.0).max(0.0);
            let available_main = match parent_content.style.direction {
                UiDirection::Row => (inner.width - gap_total).max(0.0),
                UiDirection::Column => (inner.height - gap_total).max(0.0),
            };
            let auto_main = available_main / count;
            let intrinsic = self
                .intrinsic
                .iter()
                .find(|(id, _)| *id == node)
                .map(|(_, rect)| *rect)
                .unwrap_or(UiRect::zero());
            let (mut main, mut cross) =
                match parent_content.style.direction {
                    UiDirection::Row => (
                        content.style.width.resolve(inner.width).unwrap_or(
                            if intrinsic.width > 0.0 {
                                intrinsic.width
                            } else {
                                auto_main
                            },
                        ),
                        content.style.height.resolve(inner.height).unwrap_or(
                            if intrinsic.height > 0.0 {
                                intrinsic.height
                            } else {
                                inner.height
                            },
                        ),
                    ),
                    UiDirection::Column => (
                        content.style.height.resolve(inner.height).unwrap_or(
                            if intrinsic.height > 0.0 {
                                intrinsic.height
                            } else {
                                auto_main
                            },
                        ),
                        content.style.width.resolve(inner.width).unwrap_or(
                            if intrinsic.width > 0.0 {
                                intrinsic.width
                            } else {
                                inner.width
                            },
                        ),
                    ),
                };
            let (main_min, main_max) = match parent_content.style.direction {
                UiDirection::Row => (
                    content.style.min_width.min(inner.width).max(0.0),
                    content.style.max_width.min(inner.width),
                ),
                UiDirection::Column => (
                    content.style.min_height.min(inner.height).max(0.0),
                    content.style.max_height.min(inner.height),
                ),
            };
            main = main.clamp(main_min, main_max.max(main_min));
            let (cross_min, cross_max) = match parent_content.style.direction {
                UiDirection::Row => (
                    content.style.min_height.min(inner.height).max(0.0),
                    content.style.max_height.min(inner.height),
                ),
                UiDirection::Column => (
                    content.style.min_width.min(inner.width).max(0.0),
                    content.style.max_width.min(inner.width),
                ),
            };
            cross = cross.clamp(cross_min, cross_max.max(cross_min));
            let main = main.clamp(
                0.0,
                if matches!(parent_content.style.direction, UiDirection::Row) {
                    inner.width
                } else {
                    inner.height
                },
            );
            let cross = cross.clamp(
                0.0,
                if matches!(parent_content.style.direction, UiDirection::Row) {
                    inner.height
                } else {
                    inner.width
                },
            );
            let offset = siblings
                .iter()
                .take(index)
                .try_fold(0.0f32, |sum, sibling| {
                    let rect = tree.rect(*sibling).ok()?;
                    Some(
                        sum + if matches!(parent_content.style.direction, UiDirection::Row) {
                            rect.width
                        } else {
                            rect.height
                        } + parent_content.style.gap,
                    )
                })
                .ok_or(UiError::IndexOverflow)?;
            let mut rect = match parent_content.style.direction {
                UiDirection::Row => UiRect {
                    x: inner.x + offset,
                    y: inner.y,
                    width: main,
                    height: cross,
                },
                UiDirection::Column => UiRect {
                    x: inner.x,
                    y: inner.y + offset,
                    width: cross,
                    height: main,
                },
            };
            let margin = content.style.margin;
            if let UiPosition::Absolute { left, top } = content.style.position {
                rect.x = inner.x + left + margin.left;
                rect.y = inner.y + top + margin.top;
            } else {
                rect.x += margin.left;
                rect.y += margin.top;
                rect.width = (rect.width - margin.horizontal()).max(0.0);
                rect.height = (rect.height - margin.vertical()).max(0.0);
                let cross_available = match parent_content.style.direction {
                    UiDirection::Row => inner.height,
                    UiDirection::Column => inner.width,
                };
                let cross_size = match parent_content.style.direction {
                    UiDirection::Row => rect.height,
                    UiDirection::Column => rect.width,
                };
                let cross_offset = match parent_content.style.align {
                    UiAlign::Start | UiAlign::Stretch => 0.0,
                    UiAlign::Center => (cross_available - cross_size).max(0.0) * 0.5,
                    UiAlign::End => (cross_available - cross_size).max(0.0),
                };
                match parent_content.style.direction {
                    UiDirection::Row => rect.y += cross_offset,
                    UiDirection::Column => rect.x += cross_offset,
                }
                if parent_content.style.align == UiAlign::Stretch {
                    match parent_content.style.direction {
                        UiDirection::Row => {
                            rect.height = (cross_available - margin.vertical()).max(0.0)
                        }
                        UiDirection::Column => {
                            rect.width = (cross_available - margin.horizontal()).max(0.0)
                        }
                    }
                }
                if parent_content.style.overflow == UiOverflow::Scroll {
                    rect.x -= parent_content.style.scroll_x;
                    rect.y -= parent_content.style.scroll_y;
                }
            }
            let was_dirty = tree.is_dirty(node)?;
            tree.set_rect(node, rect)?;
            stats.visited = stats.visited.saturating_add(1);
            if was_dirty {
                stats.dirty = stats.dirty.saturating_add(1);
            }
        }
        if stats.iterations > self.config.max_layout_iterations as usize {
            return Err(UiError::LayoutDidNotConverge);
        }
        diagnostics.layout_passes = diagnostics.layout_passes.saturating_add(1);
        diagnostics.dirty_nodes = stats.dirty;
        Ok(stats)
    }

    /// Measures intrinsic content from leaves toward the root. The pass is
    /// deliberately conservative: it estimates fallback glyphs at 8x16 and
    /// never allocates outside the pre-reserved node budget.
    fn measure(&mut self, tree: &UiTree) -> Result<(), UiError> {
        self.intrinsic.clear();
        for node in self.order.iter().rev().copied() {
            let content = tree.content(node)?;
            let mut width = 0.0f32;
            let mut height = 0.0f32;
            if let Some(text) = &content.text {
                let glyphs = text.as_str().chars().count().min(self.config.max_glyphs);
                width = (glyphs as f32 * 8.0).min(self.config.max_upload_bytes as f32);
                height = if glyphs == 0 { 0.0 } else { 16.0 };
            }
            let children = tree.children(node)?;
            if !children.is_empty() {
                let mut child_width = 0.0f32;
                let mut child_height = 0.0f32;
                for child in children {
                    let child_rect = self
                        .intrinsic
                        .iter()
                        .find(|(id, _)| *id == *child)
                        .map(|(_, rect)| *rect)
                        .unwrap_or(UiRect::zero());
                    match content.style.direction {
                        UiDirection::Row => {
                            child_width += child_rect.width;
                            child_height = child_height.max(child_rect.height);
                        }
                        UiDirection::Column => {
                            child_width = child_width.max(child_rect.width);
                            child_height += child_rect.height;
                        }
                    }
                }
                let gaps = content.style.gap * children.len().saturating_sub(1) as f32;
                match content.style.direction {
                    UiDirection::Row => child_width += gaps,
                    UiDirection::Column => child_height += gaps,
                }
                width = width.max(child_width);
                height = height.max(child_height);
            }
            width += content.style.padding.horizontal();
            height += content.style.padding.vertical();
            if !width.is_finite() || !height.is_finite() {
                return Err(UiError::IndexOverflow);
            }
            self.intrinsic.push((
                node,
                UiRect {
                    x: 0.0,
                    y: 0.0,
                    width: width.min(content.style.max_width),
                    height: height.min(content.style.max_height),
                },
            ));
        }
        Ok(())
    }
}
