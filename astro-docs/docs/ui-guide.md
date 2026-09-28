# Create and lay out UI

The UI subsystem owns a retained tree. Game logic sends commands; layout assigns rectangles; paint builds render-owned snapshots. A native bridge must deliver pointer coordinates in the same coordinate system as layout.

## Build a bounded tree

```rust
use rust_kernel_game_engine_kit::subsystems::ui::{
    UiConfig, UiTree, UiNodeContent, UiNodeKind,
    UiLayoutEngine, UiDiagnostics, UiRect,
};

let config = UiConfig::default();
let mut tree = UiTree::try_new(config).unwrap();
let root = tree.root();
let child = tree.create(root, UiNodeContent::new(UiNodeKind::Panel)).unwrap();
let mut layout = UiLayoutEngine::try_new(config).unwrap();
let mut diagnostics = UiDiagnostics::default();
let stats = layout.layout(&mut tree, UiRect {
    x: 0.0, y: 0.0, width: 900.0, height: 760.0,
}, &mut diagnostics).unwrap();
assert!(tree.contains(child));
assert!(stats.iterations > 0);
```

This produces layout state, not a native window. To display it, use the UI subsystem's paint/snapshot contract and a renderer/presentation path. Keep tree node count, depth, text bytes and command count within `UiConfig` budgets.

## Follow a click

1. The native event loop collects mouse/button input.
2. Input is translated to the engine/UI event representation.
3. Hit testing uses current layout, clipping, focus and capture state.
4. Application logic handles the resulting action and queues changes.
5. UI emits updated paint/snapshot data for presentation.

When clicks stop after a modal closes, inspect capture and active scene state. When a resized window misses targets, compare current client dimensions, DPI conversion, layout rectangles and pointer coordinates. Repainting cannot fix incorrect hit-test coordinates.

## Follow a visual change

Change UI state through the documented command/tree boundary, then rebuild only the required layout/paint data. A dirty upload represents changed render data; it is not a request to repeatedly destroy and recreate the native window. Avoid using whole-window invalidation to hide state bugs.

## Sudoku implementation map

`model.rs` owns puzzle rules; `game.rs` coordinates game state; `input.rs` interprets actions; `layout.rs` defines placement; `ui.rs` composes UI; `paint.rs` and `native.rs` handle example-specific drawing/presentation. Read these together when diagnosing pause/save/continue or game-over transitions.

Run `cargo test --locked --test ui_rendering` for the integration suite. Native click and resize behavior still needs an interactive Windows run.
