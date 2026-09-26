//! Render-owned clips, paint items, deterministic batches, and upload bounds.

use super::budget::{UiConfig, UiDiagnostics, UiError};
use super::components::{UiNodeKind, UiRect};
use super::id::{FontId, TextureId, UiNodeId};
use super::resources::UiResourceTable;
use super::text::{FallbackGlyphProvider, Glyph, GlyphProvider, GlyphRun};
use super::tree::UiTree;

const UI_ITEM_STRIDE_BYTES: usize = 80;
const UI_GLYPH_STRIDE_BYTES: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ClipId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiClip {
    pub id: ClipId,
    pub rect: UiRect,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PaintKind {
    Rect { color: [f32; 4] },
    Gradient { from: [f32; 4], to: [f32; 4] },
    RoundedRect { color: [f32; 4], radius: f32 },
    Border { color: [f32; 4], width: f32 },
    Image { texture: TextureId },
    GlyphRun { font: FontId, run: GlyphRun },
    Custom { token: u32 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct PaintItem {
    pub node: UiNodeId,
    pub rect: UiRect,
    pub clip: ClipId,
    pub z_index: i32,
    pub opacity: f32,
    pub kind: PaintKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct UiBatchKey {
    pub pass: u8,
    pub clip: ClipId,
    pub texture: Option<TextureId>,
    pub font: Option<FontId>,
    pub z_index: i32,
    pub kind: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiBatch {
    pub key: UiBatchKey,
    pub first: u32,
    pub count: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UiUploadPlan {
    pub item_count: u32,
    pub batch_count: u32,
    pub glyph_count: u32,
    pub upload_bytes: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiUploadRange {
    pub offset: usize,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiRenderItem {
    pub node: UiNodeId,
    pub rect: UiRect,
    pub clip_rect: UiRect,
    pub color: [f32; 4],
    pub clip: ClipId,
    pub z_index: i32,
    pub opacity: f32,
    pub kind: u8,
    pub texture: Option<TextureId>,
    pub font: Option<FontId>,
    pub glyph_count: u32,
    pub glyph_offset: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UiRenderSnapshot {
    pub items: Vec<UiRenderItem>,
    pub batches: Vec<UiBatch>,
    pub glyphs: Vec<Glyph>,
    pub upload: UiUploadPlan,
}

impl UiRenderSnapshot {
    pub fn encode_dirty_ranges(
        &self,
        previous: Option<&UiRenderSnapshot>,
    ) -> Result<Vec<UiUploadRange>, UiError> {
        let current = self.encode_bytes()?;
        let Some(previous) = previous else {
            return Ok(vec![UiUploadRange {
                offset: 0,
                bytes: current,
            }]);
        };
        let old = previous.encode_bytes()?;
        let chunk_count = current.len().max(old.len()).div_ceil(32);
        let mut ranges = Vec::new();
        ranges
            .try_reserve(chunk_count)
            .map_err(|_| UiError::AllocationFailed)?;
        let mut pending_start = None;
        for chunk in 0..chunk_count {
            let start = chunk.checked_mul(32).ok_or(UiError::IndexOverflow)?;
            let chunk_end = start.checked_add(32).ok_or(UiError::IndexOverflow)?;
            let end = chunk_end.min(current.len());
            let old_end = chunk_end.min(old.len());
            let changed = current.get(start..end) != old.get(start..old_end)
                || current.len() != old.len() && end != old_end;
            if changed && pending_start.is_none() {
                pending_start = Some(start);
            }
            let is_last = chunk + 1 == chunk_count;
            if !changed && pending_start.is_some() || changed && is_last {
                let range_start = pending_start.take().unwrap_or(start);
                let range_end = if changed && is_last {
                    current.len()
                } else {
                    start.min(current.len())
                };
                if range_start < range_end {
                    let mut bytes = Vec::new();
                    bytes
                        .try_reserve_exact(range_end - range_start)
                        .map_err(|_| UiError::AllocationFailed)?;
                    bytes.extend_from_slice(&current[range_start..range_end]);
                    ranges.push(UiUploadRange {
                        offset: range_start,
                        bytes,
                    });
                }
            }
        }
        Ok(ranges)
    }

    /// Encodes stable, backend-neutral records for visible items and glyphs.
    /// Native backends upload this stream to a frame-owned GPU buffer; shader
    /// and pipeline interpretation remains outside the retained UI tree.
    pub fn encode_bytes(&self) -> Result<Vec<u8>, UiError> {
        let bytes = self
            .items
            .len()
            .checked_mul(UI_ITEM_STRIDE_BYTES)
            .and_then(|items| {
                self.glyphs
                    .len()
                    .checked_mul(UI_GLYPH_STRIDE_BYTES)
                    .and_then(|glyphs| items.checked_add(glyphs))
            })
            .ok_or(UiError::IndexOverflow)?;
        let mut output = Vec::new();
        output
            .try_reserve_exact(bytes)
            .map_err(|_| UiError::AllocationFailed)?;
        for item in &self.items {
            for value in [item.rect.x, item.rect.y, item.rect.width, item.rect.height] {
                output.extend_from_slice(&value.to_le_bytes());
            }
            for value in [
                item.clip_rect.x,
                item.clip_rect.y,
                item.clip_rect.width,
                item.clip_rect.height,
            ] {
                output.extend_from_slice(&value.to_le_bytes());
            }
            for value in item.color {
                output.extend_from_slice(&value.to_le_bytes());
            }
            output.extend_from_slice(&item.clip.0.to_le_bytes());
            output.extend_from_slice(&item.z_index.to_le_bytes());
            output.extend_from_slice(&item.opacity.to_le_bytes());
            output.push(item.kind);
            output.push(0);
            output.extend_from_slice(&item.glyph_count.to_le_bytes());
            output.extend_from_slice(&item.glyph_offset.to_le_bytes());
            output.extend_from_slice(
                &item
                    .texture
                    .map(|id| id.index())
                    .unwrap_or(u32::MAX)
                    .to_le_bytes(),
            );
            output.extend_from_slice(
                &item
                    .font
                    .map(|id| id.index())
                    .unwrap_or(u32::MAX)
                    .to_le_bytes(),
            );
            output.resize(
                output
                    .len()
                    .checked_add(UI_ITEM_STRIDE_BYTES - 78)
                    .ok_or(UiError::IndexOverflow)?,
                0,
            );
        }
        for glyph in &self.glyphs {
            output.extend_from_slice(&glyph.codepoint.to_le_bytes());
            output.extend_from_slice(&glyph.advance.to_le_bytes());
            output.extend_from_slice(&glyph.atlas_x.to_le_bytes());
            output.extend_from_slice(&glyph.atlas_y.to_le_bytes());
            output.extend_from_slice(&glyph.width.to_le_bytes());
            output.extend_from_slice(&glyph.height.to_le_bytes());
            output.resize(
                output
                    .len()
                    .checked_add(UI_GLYPH_STRIDE_BYTES - 16)
                    .ok_or(UiError::IndexOverflow)?,
                0,
            );
        }
        debug_assert_eq!(output.len(), bytes);
        Ok(output)
    }
}

pub struct UiPaintList {
    clips: Vec<UiClip>,
    items: Vec<PaintItem>,
    batches: Vec<UiBatch>,
    node_clips: Vec<(UiNodeId, ClipId)>,
    order: Vec<UiNodeId>,
    config: UiConfig,
}

impl UiPaintList {
    pub fn try_new(config: UiConfig) -> Result<Self, UiError> {
        let mut clips = Vec::new();
        let mut items = Vec::new();
        let mut batches = Vec::new();
        let mut node_clips = Vec::new();
        let mut order = Vec::new();
        clips
            .try_reserve_exact(config.max_clips)
            .map_err(|_| UiError::AllocationFailed)?;
        items
            .try_reserve_exact(config.max_paint_items)
            .map_err(|_| UiError::AllocationFailed)?;
        batches
            .try_reserve_exact(config.max_batches)
            .map_err(|_| UiError::AllocationFailed)?;
        node_clips
            .try_reserve_exact(config.max_nodes)
            .map_err(|_| UiError::AllocationFailed)?;
        order
            .try_reserve_exact(config.max_nodes)
            .map_err(|_| UiError::AllocationFailed)?;
        Ok(Self {
            clips,
            items,
            batches,
            node_clips,
            order,
            config,
        })
    }
    pub fn clips(&self) -> &[UiClip] {
        &self.clips
    }
    pub fn items(&self) -> &[PaintItem] {
        &self.items
    }
    pub fn batches(&self) -> &[UiBatch] {
        &self.batches
    }

    pub fn snapshot(&self) -> Result<UiRenderSnapshot, UiError> {
        let mut items = Vec::new();
        let mut glyphs: Vec<Glyph> = Vec::new();
        items
            .try_reserve_exact(self.items.len())
            .map_err(|_| UiError::AllocationFailed)?;
        for item in &self.items {
            let glyph_offset = u32::try_from(glyphs.len()).map_err(|_| UiError::IndexOverflow)?;
            let (kind, texture, font, glyph_count, color) = match &item.kind {
                PaintKind::Rect { color } => (0, None, None, 0, *color),
                PaintKind::Gradient { from, .. } => (5, None, None, 0, *from),
                PaintKind::RoundedRect { color, .. } => (6, None, None, 0, *color),
                PaintKind::Border { color, .. } => (1, None, None, 0, *color),
                PaintKind::Image { texture } => (2, Some(*texture), None, 0, [1.0; 4]),
                PaintKind::GlyphRun { font, run } => {
                    glyphs
                        .try_reserve(run.glyphs().len())
                        .map_err(|_| UiError::AllocationFailed)?;
                    glyphs.extend_from_slice(run.glyphs());
                    (
                        3,
                        None,
                        Some(*font),
                        u32::try_from(run.glyphs().len()).map_err(|_| UiError::IndexOverflow)?,
                        [1.0; 4],
                    )
                }
                PaintKind::Custom { .. } => (4, None, None, 0, [1.0; 4]),
            };
            items.push(UiRenderItem {
                node: item.node,
                rect: item.rect,
                clip_rect: self
                    .clips
                    .get(item.clip.0 as usize)
                    .map(|clip| clip.rect)
                    .unwrap_or(UiRect::zero()),
                color,
                clip: item.clip,
                z_index: item.z_index,
                opacity: item.opacity,
                kind,
                texture,
                font,
                glyph_count,
                glyph_offset,
            });
        }
        let mut batches = Vec::new();
        batches
            .try_reserve_exact(self.batches.len())
            .map_err(|_| UiError::AllocationFailed)?;
        batches.extend_from_slice(&self.batches);
        let glyph_count = items.iter().try_fold(0usize, |sum, item| {
            sum.checked_add(item.glyph_count as usize)
                .ok_or(UiError::IndexOverflow)
        })?;
        let upload_bytes = items
            .len()
            .checked_mul(UI_ITEM_STRIDE_BYTES)
            .and_then(|bytes| bytes.checked_add(glyph_count.checked_mul(UI_GLYPH_STRIDE_BYTES)?))
            .ok_or(UiError::IndexOverflow)?;
        Ok(UiRenderSnapshot {
            items,
            batches,
            glyphs,
            upload: UiUploadPlan {
                item_count: u32::try_from(self.items.len()).map_err(|_| UiError::IndexOverflow)?,
                batch_count: u32::try_from(self.batches.len())
                    .map_err(|_| UiError::IndexOverflow)?,
                glyph_count: u32::try_from(glyph_count).map_err(|_| UiError::IndexOverflow)?,
                upload_bytes,
            },
        })
    }

    pub fn build(
        &mut self,
        tree: &UiTree,
        viewport: UiRect,
        diagnostics: &mut UiDiagnostics,
    ) -> Result<UiUploadPlan, UiError> {
        self.build_with_resources(tree, viewport, None, diagnostics)
    }

    pub fn build_with_resources(
        &mut self,
        tree: &UiTree,
        viewport: UiRect,
        resources: Option<&UiResourceTable>,
        diagnostics: &mut UiDiagnostics,
    ) -> Result<UiUploadPlan, UiError> {
        if !viewport.sane() {
            return Err(UiError::InvalidValue);
        }
        self.clips.clear();
        self.items.clear();
        self.batches.clear();
        self.node_clips.clear();
        tree.preorder(&mut self.order)?;
        self.push_clip(viewport)?;
        let order_len = self.order.len();
        for order_index in 0..order_len {
            let node = self.order[order_index];
            let content = tree.content(node)?;
            let rect = tree.rect(node)?;
            let parent_clip = tree
                .parent(node)
                .ok()
                .flatten()
                .and_then(|parent| {
                    self.node_clips
                        .iter()
                        .find(|(id, _)| *id == parent)
                        .map(|(_, clip)| *clip)
                })
                .unwrap_or(ClipId(0));
            let parent_rect = self
                .clips
                .get(parent_clip.0 as usize)
                .map(|clip| clip.rect)
                .unwrap_or(viewport);
            let clip_rect = parent_rect.intersect(rect);
            let clip_id = self.push_clip(clip_rect)?;
            self.node_clips.push((node, clip_id));
            if !content.visible || clip_rect.area() <= 0.0 {
                diagnostics.clipped_items = diagnostics.clipped_items.saturating_add(1);
                continue;
            }
            let kind = match content.kind {
                UiNodeKind::Root | UiNodeKind::Panel => {
                    if let Some((from, to)) = content.style.gradient {
                        PaintKind::Gradient { from, to }
                    } else if content.style.corner_radius > 0.0 {
                        PaintKind::RoundedRect {
                            color: content.style.background,
                            radius: content.style.corner_radius,
                        }
                    } else {
                        PaintKind::Rect {
                            color: content.style.background,
                        }
                    }
                }
                UiNodeKind::Text => {
                    // Missing text is a valid intermediate state while a retained tree is
                    // being hydrated. Emit an empty run instead of failing the whole frame.
                    let font = content.font.unwrap_or(FontId::new(0, 1));
                    let unavailable = resources
                        .zip(content.font)
                        .is_some_and(|(table, requested_font)| !table.font_ready(requested_font));
                    if unavailable {
                        diagnostics.overflow_fallbacks =
                            diagnostics.overflow_fallbacks.saturating_add(1);
                        PaintKind::Rect {
                            color: content.style.background,
                        }
                    } else {
                        let run = content
                            .text
                            .as_ref()
                            .map(|text| {
                                FallbackGlyphProvider::default().shape_wrapped(
                                    font,
                                    text.as_str(),
                                    self.config.max_glyphs,
                                    rect.width.max(1.0),
                                )
                            })
                            .transpose()?
                            .unwrap_or_default();
                        PaintKind::GlyphRun { font, run }
                    }
                }
                UiNodeKind::Image => content
                    .image
                    .filter(|image| {
                        resources
                            .map(|table| table.texture_ready(image.texture))
                            .unwrap_or(true)
                    })
                    .map(|image| PaintKind::Image {
                        texture: image.texture,
                    })
                    .unwrap_or_else(|| {
                        diagnostics.overflow_fallbacks =
                            diagnostics.overflow_fallbacks.saturating_add(1);
                        PaintKind::Rect {
                            color: content.style.background,
                        }
                    }),
                UiNodeKind::Custom => PaintKind::Custom {
                    token: node.index(),
                },
            };
            self.push_item(PaintItem {
                node,
                rect,
                clip: clip_id,
                z_index: content.z_index,
                opacity: content.opacity,
                kind,
            })?;
            if content.style.border_width > 0.0
                && content.style.border.iter().any(|value| *value > 0.0)
            {
                self.push_item(PaintItem {
                    node,
                    rect,
                    clip: clip_id,
                    z_index: content.z_index,
                    opacity: content.opacity,
                    kind: PaintKind::Border {
                        color: content.style.border,
                        width: content.style.border_width,
                    },
                })?;
            }
        }
        self.build_batches()?;
        let mut glyphs = 0usize;
        for item in &self.items {
            if let PaintKind::GlyphRun { run, .. } = &item.kind {
                glyphs = glyphs
                    .checked_add(run.glyphs().len())
                    .ok_or(UiError::IndexOverflow)?;
            }
        }
        let item_count = u32::try_from(self.items.len()).map_err(|_| UiError::IndexOverflow)?;
        let batch_count = u32::try_from(self.batches.len()).map_err(|_| UiError::IndexOverflow)?;
        let glyph_count = u32::try_from(glyphs).map_err(|_| UiError::IndexOverflow)?;
        let upload_bytes = self
            .items
            .len()
            .checked_mul(UI_ITEM_STRIDE_BYTES)
            .and_then(|bytes| bytes.checked_add(glyphs.checked_mul(UI_GLYPH_STRIDE_BYTES)?))
            .ok_or(UiError::IndexOverflow)?;
        if upload_bytes > self.config.max_upload_bytes {
            return Err(UiError::CapacityExceeded);
        }
        diagnostics.paint_items = self.items.len();
        diagnostics.batches = self.batches.len();
        diagnostics.glyphs = glyphs;
        diagnostics.uploaded_bytes = upload_bytes;
        Ok(UiUploadPlan {
            item_count,
            batch_count,
            glyph_count,
            upload_bytes,
        })
    }

    fn push_clip(&mut self, rect: UiRect) -> Result<ClipId, UiError> {
        if self.clips.len() >= self.config.max_clips {
            return Err(UiError::CapacityExceeded);
        }
        let id = ClipId(u32::try_from(self.clips.len()).map_err(|_| UiError::IndexOverflow)?);
        self.clips.push(UiClip { id, rect });
        Ok(id)
    }

    fn push_item(&mut self, item: PaintItem) -> Result<(), UiError> {
        if self.items.len() >= self.config.max_paint_items {
            return Err(UiError::CapacityExceeded);
        }
        self.items.push(item);
        Ok(())
    }

    fn key(item: &PaintItem) -> UiBatchKey {
        match &item.kind {
            PaintKind::Rect { .. } => UiBatchKey {
                pass: 0,
                clip: item.clip,
                texture: None,
                font: None,
                z_index: item.z_index,
                kind: 0,
            },
            PaintKind::Gradient { .. } => UiBatchKey {
                pass: 0,
                clip: item.clip,
                texture: None,
                font: None,
                z_index: item.z_index,
                kind: 5,
            },
            PaintKind::RoundedRect { .. } => UiBatchKey {
                pass: 0,
                clip: item.clip,
                texture: None,
                font: None,
                z_index: item.z_index,
                kind: 6,
            },
            PaintKind::Border { .. } => UiBatchKey {
                pass: 0,
                clip: item.clip,
                texture: None,
                font: None,
                z_index: item.z_index,
                kind: 1,
            },
            PaintKind::Image { texture } => UiBatchKey {
                pass: 0,
                clip: item.clip,
                texture: Some(*texture),
                font: None,
                z_index: item.z_index,
                kind: 2,
            },
            PaintKind::GlyphRun { font, .. } => UiBatchKey {
                pass: 1,
                clip: item.clip,
                texture: None,
                font: Some(*font),
                z_index: item.z_index,
                kind: 3,
            },
            PaintKind::Custom { .. } => UiBatchKey {
                pass: 2,
                clip: item.clip,
                texture: None,
                font: None,
                z_index: item.z_index,
                kind: 4,
            },
        }
    }

    fn build_batches(&mut self) -> Result<(), UiError> {
        for (index, item) in self.items.iter().enumerate() {
            let key = Self::key(item);
            let index = u32::try_from(index).map_err(|_| UiError::IndexOverflow)?;
            if let Some(last) = self.batches.last_mut() {
                if last.key == key && last.first.checked_add(last.count) == Some(index) {
                    last.count = last.count.saturating_add(1);
                    continue;
                }
            }
            if self.batches.len() >= self.config.max_batches {
                return Err(UiError::CapacityExceeded);
            }
            self.batches.push(UiBatch {
                key,
                first: index,
                count: 1,
            });
        }
        Ok(())
    }
}
