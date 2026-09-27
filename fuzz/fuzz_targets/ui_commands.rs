#![no_main]

use libfuzzer_sys::fuzz_target;
use rust_kernel_game_engine_kit::subsystems::ui::{
    UiCommand, UiCommandBuffer, UiConfig, UiLayoutEngine, UiNodeContent, UiNodeKind,
    UiPaintList, UiRect, UiText, UiTree,
};
use rust_kernel_game_engine_kit::subsystems::ui::UiDiagnostics;

fuzz_target!(|data: &[u8]| {
    let node_limit = 1 + usize::from(data.first().copied().unwrap_or(0) % 32);
    let config = UiConfig {
        max_nodes: node_limit,
        max_commands: 64,
        max_paint_items: node_limit.saturating_add(1),
        max_clips: node_limit.saturating_add(1),
        max_batches: node_limit.saturating_add(1),
        max_text_bytes: 256,
        max_glyphs: 256,
        max_upload_bytes: 64 * 1024,
        ..UiConfig::default()
    };
    let Ok(mut tree) = UiTree::try_new(config) else {
        return;
    };
    let Ok(mut commands) = UiCommandBuffer::try_with_capacity(config.max_commands) else {
        return;
    };

    for chunk in data.get(1..).unwrap_or_default().chunks(4).take(64) {
        let parent = tree.root();
        let selector = chunk.first().copied().unwrap_or(0) % 5;
        let command = match selector {
            0 => UiCommand::Create {
                parent,
                content: UiNodeContent::new(if chunk.get(1).copied().unwrap_or(0) & 1 == 0 {
                    UiNodeKind::Panel
                } else {
                    UiNodeKind::Text
                }),
            },
            1 => UiCommand::SetVisible {
                node: parent,
                visible: chunk.get(1).copied().unwrap_or(0) & 1 == 1,
            },
            2 => UiCommand::SetZIndex {
                node: parent,
                z_index: i32::from(chunk.get(1).copied().unwrap_or(0)),
            },
            3 => UiCommand::SetText {
                node: parent,
                text: UiText::try_new("fuzz", config.max_text_bytes).expect("bounded literal"),
            },
            _ => UiCommand::Destroy(parent),
        };
        let _ = commands.try_push(command);
    }
    let _ = commands.apply(&mut tree);

    let mut layout = match UiLayoutEngine::try_new(config) {
        Ok(layout) => layout,
        Err(_) => return,
    };
    let mut diagnostics = UiDiagnostics::default();
    let viewport = UiRect {
        x: 0.0,
        y: 0.0,
        width: 1280.0,
        height: 720.0,
    };
    let _ = layout.layout(&mut tree, viewport, &mut diagnostics);
    let mut paint = match UiPaintList::try_new(config) {
        Ok(paint) => paint,
        Err(_) => return,
    };
    let _ = paint.build(&tree, viewport, &mut diagnostics);
});
