# Examples

## `sudoku_game`

`sudoku_game` is a playable native UI Sudoku game built on the kernel. It
exercises:

- retained UI tree creation and deferred cell styling;
- bounded layout, clipping, and 9x9 board composition;
- native Vulkan/headless renderer packet extraction;
- pointer and keyboard input translation through the message bus;
- difficulty-aware generation with uniqueness checking;
- lives, timer, mistakes, notes, hints, pause, win state, stars, and score;
- game logic updating UI through deferred commands, without direct tree access.

Run the interactive UI game on Windows:

```text
cargo run --example sudoku_game --release
```

On Windows the example uses a bounded GDI presentation bridge by default, so
it remains playable without a Vulkan driver. The handwritten Vulkan path is
opt-in with `RKE_ENABLE_NATIVE_VULKAN=experimental`; a failed Vulkan bootstrap
falls back safely. Set `SUDOKU_DIFFICULTY=medium` or `hard` before launch to
choose another mode.

Controls: click a cell or a number button; click Erase, Note, Hint, Pause, or
Menu; arrows move selection; `1-9` enters digits; `N` toggles notes; `H` uses
a hint; `0`, Backspace, or Delete clears; `P`, Space, or Escape pauses.
