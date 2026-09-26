//! Bounded retained UI subsystem and render-owned UI frame contract.

pub mod budget;
pub mod commands;
pub mod components;
pub mod id;
pub mod input;
pub mod layout;
pub mod paint;
pub mod resources;
pub mod text;
pub mod tree;

use crate::kernel::{KernelContext, Subsystem};
use crate::subsystems::messages::{InputFrame, UiAssetReady, UiCommandMessage, WindowResized};

pub use budget::{UiConfig, UiDiagnostics, UiError};
pub use commands::{UiCommand, UiCommandBuffer};
pub use components::{
    UiAccessibility, UiAccessibilityRole, UiAlign, UiDirection, UiEdges, UiImage, UiLength,
    UiNodeContent, UiNodeKind, UiOverflow, UiPosition, UiRect, UiStyle, UiText,
};
pub use id::{FontId, StyleId, TextureId, UiNodeId};
pub use input::{UiFocusCapture, UiInputEvent, UiInputResult};
pub use layout::{LayoutStats, UiLayoutEngine};
pub use paint::{
    ClipId, PaintItem, UiBatch, UiBatchKey, UiPaintList, UiRenderSnapshot, UiUploadPlan,
    UiUploadRange,
};
pub use resources::{UiAssetState, UiResourceTable};
pub use text::{FallbackGlyphProvider, Glyph, GlyphAtlas, GlyphProvider, GlyphRun};
pub use tree::UiTree;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UiFrameReady {
    pub nodes: usize,
    pub paint_items: usize,
    pub batches: usize,
    pub glyphs: usize,
    pub uploaded_bytes: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UiFramePacket {
    pub ready: UiFrameReady,
    pub snapshot: UiRenderSnapshot,
    pub upload_ranges: Vec<UiUploadRange>,
}

pub struct UiSubsystem {
    config: UiConfig,
    tree: UiTree,
    commands: UiCommandBuffer,
    layout: UiLayoutEngine,
    paint: UiPaintList,
    resources: UiResourceTable,
    focus: UiFocusCapture,
    diagnostics: UiDiagnostics,
    viewport: UiRect,
    last_frame: UiFrameReady,
    last_packet: Option<UiFramePacket>,
    layout_scratch: LayoutStats,
}

impl UiSubsystem {
    pub fn try_new(config: UiConfig) -> Result<Self, UiError> {
        if !config.dpi_scale.is_finite() || config.dpi_scale <= 0.0 {
            return Err(UiError::InvalidValue);
        }
        Ok(Self {
            tree: UiTree::try_new(config)?,
            commands: UiCommandBuffer::try_with_capacity(config.max_commands)?,
            layout: UiLayoutEngine::try_new(config)?,
            paint: UiPaintList::try_new(config)?,
            resources: UiResourceTable::try_new(config)?,
            config,
            focus: UiFocusCapture::default(),
            diagnostics: UiDiagnostics::default(),
            viewport: UiRect {
                x: 0.0,
                y: 0.0,
                width: 1280.0,
                height: 720.0,
            },
            last_frame: UiFrameReady::default(),
            last_packet: None,
            layout_scratch: LayoutStats::default(),
        })
    }
    pub fn new() -> Self {
        Self::try_new(UiConfig::default()).expect("UI subsystem allocation failed")
    }
    pub fn tree(&self) -> &UiTree {
        &self.tree
    }
    pub fn tree_mut(&mut self) -> &mut UiTree {
        &mut self.tree
    }
    pub fn commands(&self) -> &UiCommandBuffer {
        &self.commands
    }
    pub fn commands_mut(&mut self) -> &mut UiCommandBuffer {
        &mut self.commands
    }
    pub fn resources(&self) -> &UiResourceTable {
        &self.resources
    }
    pub fn resources_mut(&mut self) -> &mut UiResourceTable {
        &mut self.resources
    }
    pub fn paint(&self) -> &UiPaintList {
        &self.paint
    }
    pub fn focus(&self) -> UiFocusCapture {
        self.focus
    }
    pub fn diagnostics(&self) -> UiDiagnostics {
        self.diagnostics
    }
    pub fn last_frame(&self) -> UiFrameReady {
        self.last_frame
    }
    pub fn last_packet(&self) -> Option<&UiFramePacket> {
        self.last_packet.as_ref()
    }
    pub fn config(&self) -> UiConfig {
        self.config
    }
    pub fn route_input(&mut self, event: UiInputEvent) -> UiInputResult {
        self.focus.route(&self.tree, event)
    }
}

impl Default for UiSubsystem {
    fn default() -> Self {
        Self::new()
    }
}

impl Subsystem for UiSubsystem {
    fn name(&self) -> &'static str {
        "ui"
    }
    fn dependencies(&self) -> &'static [&'static str] {
        &["window", "input"]
    }
    fn init(&mut self, _ctx: &mut KernelContext<'_>) {}
    fn tick(&mut self, ctx: &mut KernelContext<'_>, _dt_ns: u64) {
        let mut resized = None;
        for envelope in ctx.receive() {
            if envelope.topic == 1 {
                if let Ok(size) = envelope.downcast::<WindowResized>() {
                    resized = Some((size.width, size.height));
                }
            } else if envelope.topic == 2 {
                if let Ok(input) = envelope.downcast::<InputFrame>() {
                    let scale = self.config.dpi_scale;
                    let x = (input.mouse_x as f32 / scale) as i32;
                    let y = (input.mouse_y as f32 / scale) as i32;
                    let _ = self
                        .focus
                        .route(&self.tree, UiInputEvent::PointerMove { x, y });
                    for button in 0..16u8 {
                        let mask = 1u16 << button;
                        if input.pressed_buttons & mask != 0 {
                            let _ = self.focus.route(
                                &self.tree,
                                UiInputEvent::PointerButton {
                                    button,
                                    pressed: true,
                                    x,
                                    y,
                                },
                            );
                        }
                        if input.released_buttons & mask != 0 {
                            let _ = self.focus.route(
                                &self.tree,
                                UiInputEvent::PointerButton {
                                    button,
                                    pressed: false,
                                    x,
                                    y,
                                },
                            );
                        }
                    }
                    for key in input
                        .pressed_keys
                        .iter()
                        .take(usize::from(input.pressed_key_count))
                    {
                        let _ = self.focus.route(
                            &self.tree,
                            UiInputEvent::Key {
                                key: *key,
                                pressed: true,
                            },
                        );
                    }
                    for key in input
                        .released_keys
                        .iter()
                        .take(usize::from(input.released_key_count))
                    {
                        let _ = self.focus.route(
                            &self.tree,
                            UiInputEvent::Key {
                                key: *key,
                                pressed: false,
                            },
                        );
                    }
                    for byte in input.text.iter().take(usize::from(input.text_count)) {
                        let _ = self
                            .focus
                            .route(&self.tree, UiInputEvent::TextByte { byte: *byte });
                    }
                }
            } else if envelope.topic == 3 {
                if let Ok(asset) = envelope.downcast::<UiAssetReady>() {
                    let state = match asset.state {
                        0 => Some(UiAssetState::Missing),
                        1 => Some(UiAssetState::Loading),
                        2 => Some(UiAssetState::Ready),
                        3 => Some(UiAssetState::Failed),
                        _ => None,
                    };
                    if let Some(state) = state {
                        let result = if asset.kind == 0 {
                            self.resources.set_texture_state(
                                TextureId::new(asset.index, asset.generation),
                                state,
                            )
                        } else if asset.kind == 1 {
                            self.resources
                                .set_font_state(FontId::new(asset.index, asset.generation), state)
                        } else {
                            Err(UiError::InvalidValue)
                        };
                        if result.is_err() {
                            self.diagnostics.record_error();
                        }
                    } else {
                        self.diagnostics.record_error();
                    }
                }
            } else if envelope.topic == 4 {
                if let Ok(message) = envelope.downcast::<UiCommandMessage>() {
                    if self.commands.try_push(message.0).is_err() {
                        self.diagnostics.record_error();
                    }
                }
            }
        }
        if let Some((width, height)) = resized {
            if width > 0 && height > 0 {
                self.viewport.width = width as f32 / self.config.dpi_scale;
                self.viewport.height = height as f32 / self.config.dpi_scale;
            }
        }
        let _ = self.commands.apply(&mut self.tree);
        self.focus.clear_stale(&self.tree);
        let layout = self
            .layout
            .layout(&mut self.tree, self.viewport, &mut self.diagnostics);
        if let Ok(stats) = layout {
            self.layout_scratch = stats;
        }
        match self.paint.build_with_resources(
            &self.tree,
            self.viewport,
            Some(&self.resources),
            &mut self.diagnostics,
        ) {
            Ok(plan) => {
                self.last_frame = UiFrameReady {
                    nodes: self.tree.len(),
                    paint_items: plan.item_count as usize,
                    batches: plan.batch_count as usize,
                    glyphs: plan.glyph_count as usize,
                    uploaded_bytes: plan.upload_bytes,
                };
                if let Ok(snapshot) = self.paint.snapshot() {
                    let upload_ranges = snapshot
                        .encode_dirty_ranges(
                            self.last_packet.as_ref().map(|packet| &packet.snapshot),
                        )
                        .unwrap_or_default();
                    self.last_packet = Some(UiFramePacket {
                        ready: self.last_frame,
                        snapshot,
                        upload_ranges,
                    });
                }
            }
            Err(_) => {
                self.diagnostics.record_error();
                self.last_frame = UiFrameReady {
                    nodes: self.tree.len(),
                    ..UiFrameReady::default()
                };
                self.last_packet = None;
            }
        }
        if let Some(renderer) = ctx.resolve("renderer") {
            if let Some(packet) = self.last_packet.clone() {
                let _ = ctx.publish(renderer, 2, packet);
            }
        }
    }
    fn shutdown(&mut self, _ctx: &mut KernelContext<'_>) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_tree_rejects_cycles_and_stale_handles() {
        let mut tree = UiTree::try_new(UiConfig {
            max_nodes: 4,
            ..UiConfig::default()
        })
        .unwrap();
        let child = tree
            .create(tree.root(), UiNodeContent::new(UiNodeKind::Panel))
            .unwrap();
        assert_eq!(
            tree.reparent(tree.root(), child),
            Err(UiError::InvalidParent)
        );
        tree.destroy(child).unwrap();
        assert!(!tree.contains(child));
        assert_eq!(tree.destroy(child), Err(UiError::InvalidNode));
    }

    #[test]
    fn layout_paint_and_batches_are_bounded() {
        let config = UiConfig {
            max_nodes: 8,
            max_paint_items: 8,
            max_clips: 8,
            max_batches: 8,
            ..UiConfig::default()
        };
        let mut tree = UiTree::try_new(config).unwrap();
        let mut panel = UiNodeContent::new(UiNodeKind::Panel);
        panel.style.width = UiLength::Percent(100.0);
        panel.style.height = UiLength::Percent(100.0);
        panel.style.background = [1.0, 0.0, 0.0, 1.0];
        tree.create(tree.root(), panel).unwrap();
        let mut layout = UiLayoutEngine::try_new(config).unwrap();
        let mut diagnostics = UiDiagnostics::default();
        layout
            .layout(
                &mut tree,
                UiRect {
                    x: 0.0,
                    y: 0.0,
                    width: 640.0,
                    height: 480.0,
                },
                &mut diagnostics,
            )
            .unwrap();
        let mut paint = UiPaintList::try_new(config).unwrap();
        let plan = paint
            .build(
                &tree,
                UiRect {
                    x: 0.0,
                    y: 0.0,
                    width: 640.0,
                    height: 480.0,
                },
                &mut diagnostics,
            )
            .unwrap();
        assert_eq!(plan.item_count, 2);
        assert_eq!(plan.batch_count, 2);
    }

    #[test]
    fn missing_resources_use_bounded_placeholders() {
        let config = UiConfig {
            max_nodes: 4,
            max_clips: 8,
            max_paint_items: 8,
            max_batches: 8,
            ..UiConfig::default()
        };
        let mut tree = UiTree::try_new(config).unwrap();
        tree.create(tree.root(), UiNodeContent::new(UiNodeKind::Text))
            .unwrap();
        tree.create(tree.root(), UiNodeContent::new(UiNodeKind::Image))
            .unwrap();
        let mut layout = UiLayoutEngine::try_new(config).unwrap();
        let mut diagnostics = UiDiagnostics::default();
        let viewport = UiRect {
            x: 0.0,
            y: 0.0,
            width: 640.0,
            height: 480.0,
        };
        layout
            .layout(&mut tree, viewport, &mut diagnostics)
            .unwrap();
        let mut paint = UiPaintList::try_new(config).unwrap();
        let plan = paint.build(&tree, viewport, &mut diagnostics).unwrap();
        assert_eq!(plan.item_count, 3);
        assert!(paint.items().iter().all(|item| item.rect.sane()));
    }

    #[test]
    fn nested_layout_resizes_deterministically() {
        let config = UiConfig {
            max_nodes: 8,
            max_clips: 16,
            max_paint_items: 16,
            max_batches: 16,
            ..UiConfig::default()
        };
        let mut tree = UiTree::try_new(config).unwrap();
        let mut parent_content = UiNodeContent::new(UiNodeKind::Panel);
        parent_content.style.direction = components::UiDirection::Row;
        parent_content.style.gap = 4.0;
        let parent = tree.create(tree.root(), parent_content).unwrap();
        let mut child = UiNodeContent::new(UiNodeKind::Panel);
        child.style.width = UiLength::Percent(50.0);
        child.style.height = UiLength::Percent(100.0);
        let first = tree.create(parent, child.clone()).unwrap();
        let second = tree.create(parent, child).unwrap();
        let mut layout = UiLayoutEngine::try_new(config).unwrap();
        let mut diagnostics = UiDiagnostics::default();
        layout
            .layout(
                &mut tree,
                UiRect {
                    x: 0.0,
                    y: 0.0,
                    width: 800.0,
                    height: 600.0,
                },
                &mut diagnostics,
            )
            .unwrap();
        let first_rect = tree.rect(first).unwrap();
        let second_rect = tree.rect(second).unwrap();
        assert!(first_rect.x < second_rect.x);
        assert!(first_rect.width.is_finite() && second_rect.width.is_finite());
        assert!(diagnostics.dirty_nodes >= 2);
    }

    #[test]
    fn accessibility_metadata_is_bounded_and_deferred() {
        let config = UiConfig {
            max_nodes: 4,
            max_commands: 4,
            max_text_bytes: 32,
            ..UiConfig::default()
        };
        let mut tree = UiTree::try_new(config).unwrap();
        let node = tree
            .create(tree.root(), UiNodeContent::new(UiNodeKind::Panel))
            .unwrap();
        let mut commands = UiCommandBuffer::try_with_capacity(config.max_commands).unwrap();
        commands
            .try_push(UiCommand::SetAccessibility {
                node,
                accessibility: UiAccessibility {
                    role: UiAccessibilityRole::Button,
                    label: Some(UiText::try_new("Play", config.max_text_bytes).unwrap()),
                    hidden: false,
                },
            })
            .unwrap();
        assert_eq!(commands.apply(&mut tree), 1);
        let content = tree.content(node).unwrap();
        assert_eq!(content.accessibility.role, UiAccessibilityRole::Button);
        assert_eq!(
            content.accessibility.label.as_ref().map(UiText::as_str),
            Some("Play")
        );
    }
}
