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

## `rubik_3d`

`rubik_3d` is the native GPU 3D bring-up example. It creates the Rubik scene,
submits the bounded render-world packet, and on Windows opens the native Vulkan
presentation path.

Run it with the default bounded test lifetime (600 frames):

```powershell
cargo run --example rubik_3d --release
```

The frame limit is intentionally bounded so CI and local diagnostics do not
leave a native window process running forever. Override it when needed:

```powershell
$env:RKE_RUBIK_MAX_FRAMES = "600"
cargo run --example rubik_3d --release
```

Set `RKE_RUBIK_MAX_FRAMES` to a larger value for a longer manual GPU run. A
value of `0` exits immediately after initialization.

### GPU workflow

`.github/workflows/gpu-performance.yml` runs automatically on every branch push
using the free `windows-latest` runner. The PR label `area: gpu` is only a
classification label; it does not select or provision a GitHub runner. The
free lane records the adapter, runs the Criterion 18M/60/120 CPU baseline,
builds the native Rubik example, and validates the software/native graphics
fallback. A real GPU timing result is only asserted when the selected runner
provides a GPU/driver-backed Vulkan path; otherwise the artifact explicitly
reports `free-software-validation`.

Manual dispatch can select a real runner label when one is available by
changing `runner_label`; the default remains `windows-latest` so normal branch
CI never waits for a paid larger GPU runner.
