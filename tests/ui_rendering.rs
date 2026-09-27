use rust_kernel_game_engine_kit::subsystems::ui::{
    FontId, UiAssetState, UiConfig, UiDiagnostics, UiError, UiFocusCapture, UiInputEvent,
    UiLayoutEngine, UiLength, UiNodeContent, UiNodeKind, UiOverflow, UiPaintList, UiRect,
    UiResourceTable, UiText, UiTree,
};

#[test]
fn snapshot_wire_size_matches_declared_upload_budget() {
    let config = UiConfig {
        max_nodes: 8,
        max_clips: 16,
        max_paint_items: 16,
        max_batches: 16,
        max_upload_bytes: 64 * 1024,
        ..UiConfig::default()
    };
    let mut tree = UiTree::try_new(config).unwrap();
    let mut text = UiNodeContent::new(UiNodeKind::Text);
    text.text = Some(UiText::try_new("hello", config.max_text_bytes).unwrap());
    tree.create(tree.root(), text).unwrap();
    let viewport = UiRect {
        x: 0.0,
        y: 0.0,
        width: 640.0,
        height: 480.0,
    };
    let mut diagnostics = UiDiagnostics::default();
    UiLayoutEngine::try_new(config)
        .unwrap()
        .layout(&mut tree, viewport, &mut diagnostics)
        .unwrap();
    let mut paint = UiPaintList::try_new(config).unwrap();
    paint.build(&tree, viewport, &mut diagnostics).unwrap();
    let snapshot = paint.snapshot().unwrap();
    let bytes = snapshot.encode_bytes().unwrap();
    assert_eq!(bytes.len(), snapshot.upload.upload_bytes);
    assert_eq!(snapshot.glyphs.len(), snapshot.upload.glyph_count as usize);
    assert_eq!(
        bytes.len(),
        snapshot.items.len() * 80 + snapshot.glyphs.len() * 32
    );
    assert!(snapshot.items.iter().all(|item| item.clip_rect.sane()));
    let first = snapshot
        .items
        .first()
        .expect("text node produces a paint item");
    assert_eq!(
        f32::from_le_bytes(bytes[32..36].try_into().unwrap()),
        first.color[0]
    );
    assert_eq!(
        f32::from_le_bytes(bytes[44..48].try_into().unwrap()),
        first.color[3]
    );
    let clip_x = f32::from_le_bytes(bytes[16..20].try_into().unwrap());
    let clip_y = f32::from_le_bytes(bytes[20..24].try_into().unwrap());
    let clip_width = f32::from_le_bytes(bytes[24..28].try_into().unwrap());
    let clip_height = f32::from_le_bytes(bytes[28..32].try_into().unwrap());
    assert_eq!(
        [clip_x, clip_y, clip_width, clip_height],
        [0.0, 0.0, 640.0, 480.0]
    );
}

#[test]
fn paint_budget_accounts_for_the_full_native_item_stride() {
    let config = UiConfig {
        max_nodes: 4,
        max_clips: 8,
        max_paint_items: 8,
        max_batches: 8,
        max_upload_bytes: 79,
        ..UiConfig::default()
    };
    let mut tree = UiTree::try_new(config).unwrap();
    tree.create(tree.root(), UiNodeContent::new(UiNodeKind::Panel))
        .unwrap();
    let viewport = UiRect {
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 100.0,
    };
    let mut diagnostics = UiDiagnostics::default();
    UiLayoutEngine::try_new(config)
        .unwrap()
        .layout(&mut tree, viewport, &mut diagnostics)
        .unwrap();
    let mut paint = UiPaintList::try_new(config).unwrap();
    assert_eq!(
        paint.build(&tree, viewport, &mut diagnostics),
        Err(UiError::CapacityExceeded)
    );
}

#[test]
fn scroll_container_moves_flow_children_and_keeps_rects_finite() {
    let config = UiConfig {
        max_nodes: 8,
        max_clips: 16,
        max_paint_items: 16,
        max_batches: 16,
        ..UiConfig::default()
    };
    let mut tree = UiTree::try_new(config).unwrap();
    let mut container = UiNodeContent::new(UiNodeKind::Panel);
    container.style.height = UiLength::Points(64.0);
    container.style.overflow = UiOverflow::Scroll;
    container.style.scroll_y = 12.0;
    let parent = tree.create(tree.root(), container).unwrap();
    let mut child = UiNodeContent::new(UiNodeKind::Panel);
    child.style.height = UiLength::Points(32.0);
    let child = tree.create(parent, child).unwrap();
    let viewport = UiRect {
        x: 0.0,
        y: 0.0,
        width: 320.0,
        height: 200.0,
    };
    let mut diagnostics = UiDiagnostics::default();
    UiLayoutEngine::try_new(config)
        .unwrap()
        .layout(&mut tree, viewport, &mut diagnostics)
        .unwrap();
    let rect = tree.rect(child).unwrap();
    assert_eq!(rect.y, -12.0);
    assert!(rect.sane());
}

#[test]
fn invalid_dpi_scale_is_rejected_before_ui_allocation() {
    let config = UiConfig {
        dpi_scale: 0.0,
        ..UiConfig::default()
    };
    assert!(UiTree::try_new(config).is_ok());
    assert!(rust_kernel_game_engine_kit::subsystems::ui::UiSubsystem::try_new(config).is_err());
}

#[test]
fn unavailable_font_is_a_placeholder_until_asset_ready() {
    let config = UiConfig {
        max_nodes: 8,
        max_clips: 16,
        max_paint_items: 16,
        max_batches: 16,
        ..UiConfig::default()
    };
    let font = FontId::new(7, 1);
    let mut tree = UiTree::try_new(config).unwrap();
    let mut text = UiNodeContent::new(UiNodeKind::Text);
    text.font = Some(font);
    text.text = Some(UiText::try_new("late", config.max_text_bytes).unwrap());
    tree.create(tree.root(), text).unwrap();
    let viewport = UiRect {
        x: 0.0,
        y: 0.0,
        width: 320.0,
        height: 200.0,
    };
    let mut layout = UiLayoutEngine::try_new(config).unwrap();
    let mut diagnostics = UiDiagnostics::default();
    layout
        .layout(&mut tree, viewport, &mut diagnostics)
        .unwrap();
    let mut resources = UiResourceTable::try_new(config).unwrap();
    resources
        .register_font(font, UiAssetState::Loading)
        .unwrap();
    let mut paint = UiPaintList::try_new(config).unwrap();
    paint
        .build_with_resources(&tree, viewport, Some(&resources), &mut diagnostics)
        .unwrap();
    assert_eq!(paint.items().len(), 2);
    assert!(diagnostics.overflow_fallbacks > 0);

    resources.set_font_state(font, UiAssetState::Ready).unwrap();
    paint
        .build_with_resources(&tree, viewport, Some(&resources), &mut diagnostics)
        .unwrap();
    assert!(paint.items().iter().any(|item| matches!(
        item.kind,
        rust_kernel_game_engine_kit::subsystems::ui::paint::PaintKind::GlyphRun { .. }
    )));
}

#[test]
fn unchanged_snapshot_has_no_dirty_ranges_and_new_snapshot_is_one_range() {
    let config = UiConfig {
        max_nodes: 4,
        max_clips: 8,
        max_paint_items: 8,
        max_batches: 8,
        ..UiConfig::default()
    };
    let mut tree = UiTree::try_new(config).unwrap();
    tree.create(tree.root(), UiNodeContent::new(UiNodeKind::Panel))
        .unwrap();
    let viewport = UiRect {
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 100.0,
    };
    let mut diagnostics = UiDiagnostics::default();
    UiLayoutEngine::try_new(config)
        .unwrap()
        .layout(&mut tree, viewport, &mut diagnostics)
        .unwrap();
    let mut paint = UiPaintList::try_new(config).unwrap();
    paint.build(&tree, viewport, &mut diagnostics).unwrap();
    let snapshot = paint.snapshot().unwrap();
    assert!(snapshot
        .encode_dirty_ranges(Some(&snapshot))
        .unwrap()
        .is_empty());
    let ranges = snapshot.encode_dirty_ranges(None).unwrap();
    assert_eq!(ranges.len(), 1);
    assert_eq!(ranges[0].offset, 0);
    assert_eq!(ranges[0].bytes.len(), snapshot.upload.upload_bytes);
}

#[test]
fn nested_clip_intersection_is_carried_into_render_snapshot() {
    let config = UiConfig {
        max_nodes: 8,
        max_clips: 16,
        max_paint_items: 16,
        max_batches: 16,
        ..UiConfig::default()
    };
    let mut tree = UiTree::try_new(config).unwrap();
    let mut parent_content = UiNodeContent::new(UiNodeKind::Panel);
    parent_content.style.width = UiLength::Points(100.0);
    parent_content.style.height = UiLength::Points(40.0);
    parent_content.style.overflow = UiOverflow::Clip;
    let parent = tree.create(tree.root(), parent_content).unwrap();
    let mut child_content = UiNodeContent::new(UiNodeKind::Panel);
    child_content.style.width = UiLength::Points(200.0);
    child_content.style.height = UiLength::Points(80.0);
    let child = tree.create(parent, child_content).unwrap();
    let viewport = UiRect {
        x: 0.0,
        y: 0.0,
        width: 320.0,
        height: 200.0,
    };
    let mut diagnostics = UiDiagnostics::default();
    UiLayoutEngine::try_new(config)
        .unwrap()
        .layout(&mut tree, viewport, &mut diagnostics)
        .unwrap();
    let mut paint = UiPaintList::try_new(config).unwrap();
    paint.build(&tree, viewport, &mut diagnostics).unwrap();
    let snapshot = paint.snapshot().unwrap();
    let item = snapshot
        .items
        .iter()
        .find(|item| item.node == child)
        .unwrap();
    let parent_rect = tree.rect(parent).unwrap();
    assert!(item.clip_rect.sane());
    assert!(item.clip_rect.width <= parent_rect.width);
    assert!(item.clip_rect.height <= parent_rect.height);
}

#[test]
fn dirty_upload_aligns_changed_record_to_bounded_patch() {
    let config = UiConfig {
        max_nodes: 4,
        max_clips: 8,
        max_paint_items: 8,
        max_batches: 8,
        ..UiConfig::default()
    };
    let mut tree = UiTree::try_new(config).unwrap();
    tree.create(tree.root(), UiNodeContent::new(UiNodeKind::Panel))
        .unwrap();
    let viewport = UiRect {
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 100.0,
    };
    let mut diagnostics = UiDiagnostics::default();
    UiLayoutEngine::try_new(config)
        .unwrap()
        .layout(&mut tree, viewport, &mut diagnostics)
        .unwrap();
    let mut paint = UiPaintList::try_new(config).unwrap();
    paint.build(&tree, viewport, &mut diagnostics).unwrap();
    let baseline = paint.snapshot().unwrap();
    let mut changed = baseline.clone();
    changed.items[0].opacity = 0.5;
    let ranges = changed.encode_dirty_ranges(Some(&baseline)).unwrap();
    assert_eq!(ranges.len(), 1);
    assert_eq!(ranges[0].offset, 32);
    assert_eq!(ranges[0].bytes.len(), 32);
}

#[test]
fn input_hit_test_respects_clip_and_z_order() {
    let config = UiConfig {
        max_nodes: 8,
        ..UiConfig::default()
    };
    let mut tree = UiTree::try_new(config).unwrap();
    let mut clipped_content = UiNodeContent::new(UiNodeKind::Panel);
    clipped_content.style.overflow = UiOverflow::Clip;
    clipped_content.style.width = UiLength::Points(100.0);
    clipped_content.style.height = UiLength::Points(100.0);
    let clipped = tree.create(tree.root(), clipped_content).unwrap();
    let child = tree
        .create(clipped, UiNodeContent::new(UiNodeKind::Panel))
        .unwrap();
    let mut lower = UiNodeContent::new(UiNodeKind::Panel);
    lower.z_index = 1;
    let lower = tree.create(tree.root(), lower).unwrap();
    let mut higher = UiNodeContent::new(UiNodeKind::Panel);
    higher.z_index = 2;
    let higher = tree.create(tree.root(), higher).unwrap();
    let viewport = UiRect {
        x: 0.0,
        y: 0.0,
        width: 320.0,
        height: 200.0,
    };
    tree.content_mut(tree.root()).unwrap().interaction.hit_test = false;
    tree.set_rect(tree.root(), viewport).unwrap();
    tree.set_rect(
        clipped,
        UiRect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        },
    )
    .unwrap();
    tree.set_rect(
        child,
        UiRect {
            x: 80.0,
            y: 10.0,
            width: 80.0,
            height: 40.0,
        },
    )
    .unwrap();
    let overlap = UiRect {
        x: 160.0,
        y: 0.0,
        width: 100.0,
        height: 100.0,
    };
    tree.set_rect(lower, overlap).unwrap();
    tree.set_rect(higher, overlap).unwrap();

    let mut focus = UiFocusCapture::default();
    let clipped_result = focus.route(&tree, UiInputEvent::PointerMove { x: 150, y: 20 });
    assert_eq!(clipped_result.target, None);
    let top_result = focus.route(&tree, UiInputEvent::PointerMove { x: 180, y: 20 });
    assert_eq!(top_result.target, Some(higher));

    tree.content_mut(higher).unwrap().interaction.focusable = true;
    let pressed = focus.route(
        &tree,
        UiInputEvent::PointerButton {
            button: 0,
            pressed: true,
            x: 180,
            y: 20,
        },
    );
    assert_eq!(pressed.target, Some(higher));
    assert_eq!(pressed.focused, Some(higher));
    assert_eq!(focus.captured(), Some(higher));
}

#[test]
fn deterministic_style_property_harness_keeps_layout_and_upload_bounded() {
    let config = UiConfig {
        max_nodes: 64,
        max_clips: 128,
        max_paint_items: 128,
        max_batches: 128,
        max_upload_bytes: 128 * 1024,
        ..UiConfig::default()
    };
    let viewport = UiRect {
        x: 0.0,
        y: 0.0,
        width: 1_920.0,
        height: 1_080.0,
    };
    let mut state = 0xC0DE_CAFE_u32;
    for _case in 0..32 {
        let mut tree = UiTree::try_new(config).unwrap();
        for _ in 0..24 {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let mut panel = UiNodeContent::new(UiNodeKind::Panel);
            panel.style.width = UiLength::Points((state % 800 + 1) as f32);
            state = state.rotate_left(7);
            panel.style.height = UiLength::Points((state % 400 + 1) as f32);
            state = state.rotate_left(11);
            panel.style.gap = (state % 16) as f32;
            panel.style.background = [
                (state & 0xff) as f32 / 255.0,
                ((state >> 8) & 0xff) as f32 / 255.0,
                ((state >> 16) & 0xff) as f32 / 255.0,
                1.0,
            ];
            tree.create(tree.root(), panel).unwrap();
        }
        let mut diagnostics = UiDiagnostics::default();
        UiLayoutEngine::try_new(config)
            .unwrap()
            .layout(&mut tree, viewport, &mut diagnostics)
            .unwrap();
        let mut paint = UiPaintList::try_new(config).unwrap();
        paint.build(&tree, viewport, &mut diagnostics).unwrap();
        let snapshot = paint.snapshot().unwrap();
        assert!(snapshot.items.iter().all(|item| item.rect.sane()));
        assert!(snapshot.items.iter().all(|item| item.clip_rect.sane()));
        assert_eq!(
            snapshot.encode_bytes().unwrap().len(),
            snapshot.upload.upload_bytes
        );
        assert!(snapshot.upload.upload_bytes <= config.max_upload_bytes);
    }
}
