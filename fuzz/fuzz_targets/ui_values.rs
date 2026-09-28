#![no_main]

use libfuzzer_sys::fuzz_target;
use rust_kernel_game_engine_kit::subsystems::ui::{
    UiConfig, UiDiagnostics, UiEdges, UiLayoutEngine, UiLength, UiNodeContent, UiNodeKind,
    UiOverflow, UiPaintList, UiRect, UiTree,
};

fn byte(data: &[u8], index: usize) -> u8 {
    data.get(index).copied().unwrap_or_default()
}

fuzz_target!(|data: &[u8]| {
    let node_count = usize::from(byte(data, 0) % 32);
    let config = UiConfig {
        max_nodes: node_count.saturating_add(1).max(2),
        max_depth: 32,
        max_paint_items: node_count.saturating_add(1).max(2),
        max_clips: node_count.saturating_add(2).max(3),
        max_batches: node_count.saturating_add(1).max(2),
        max_text_bytes: 256,
        max_glyphs: 256,
        max_upload_bytes: 64 * 1024,
        ..UiConfig::default()
    };
    let Ok(mut tree) = UiTree::try_new(config) else {
        return;
    };

    for index in 0..node_count {
        let offset = 1 + index.saturating_mul(4);
        let mut content = UiNodeContent::new(if byte(data, offset) & 1 == 0 {
            UiNodeKind::Panel
        } else {
            UiNodeKind::Text
        });
        let width = f32::from(byte(data, offset + 1)) * 4.0;
        let height = f32::from(byte(data, offset + 2)) * 2.0;
        let edge = f32::from(byte(data, offset + 3) % 16);
        content.style.width = UiLength::Points(width);
        content.style.height = UiLength::Points(height);
        content.style.padding = UiEdges::all(edge);
        content.style.overflow = match byte(data, offset) % 3 {
            0 => UiOverflow::Visible,
            1 => UiOverflow::Clip,
            _ => UiOverflow::Scroll,
        };
        content.style.scroll_x = f32::from(byte(data, offset + 1) % 64);
        content.style.scroll_y = f32::from(byte(data, offset + 2) % 64);
        content.opacity = f32::from(byte(data, offset + 3)) / 255.0;
        let _ = tree.create(tree.root(), content);
    }

    let viewport = UiRect {
        x: 0.0,
        y: 0.0,
        width: 1280.0,
        height: 720.0,
    };
    let mut diagnostics = UiDiagnostics::default();
    let Ok(mut layout) = UiLayoutEngine::try_new(config) else {
        return;
    };
    let _ = layout.layout(&mut tree, viewport, &mut diagnostics);
    let Ok(mut paint) = UiPaintList::try_new(config) else {
        return;
    };
    let _ = paint.build(&tree, viewport, &mut diagnostics);
    let _ = paint.snapshot().and_then(|snapshot| snapshot.encode_bytes());
});
