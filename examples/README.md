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

The normal CI runners use CPU/software Vulkan. The real GPU workload is kept in
`.github/workflows/gpu-performance.yml` and is manual-only. Configure a
GitHub-hosted larger GPU runner or a self-hosted Windows runner with a custom
label such as `gpu-windows`, then run the workflow from the Actions tab and
enter that exact label. The workflow records the adapter, runs the Criterion
18M/60/120 CPU baseline on that host, builds the native Rubik example, runs the
bounded frame workload on the GPU, and uploads both the HTML baseline report
and runtime log. The Criterion report measures CPU preparation; the Rubik FPS
line measures the native GPU presentation path.
