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

## `gpu_stress_3d`

`gpu_stress_3d` is an interactive Minecraft-style voxel world. It generates a
height-mapped block terrain, uploads every block as a mesh instance, and opens
the native Vulkan presentation path on Windows. The scene is intentionally
separate from the Rubik demo and is used to stress cube geometry, instance
uploads, depth testing, player movement, camera movement, and frame pacing.

Run it locally:

```powershell
$env:RKE_GPU_STRESS_BLOCKS = "16384"
$env:RKE_GPU_STRESS_MAX_FRAMES = "600"
cargo run --release --example gpu_stress_3d
```

Increase the terrain workload for a heavier render:

```powershell
$env:RKE_GPU_STRESS_BLOCKS = "100000"
cargo run --release --features native-accel --example gpu_stress_3d
```

Controls:

- left/right drag: orbit the world camera;
- `H`: regenerate the stress scene;
- `N`: reset the camera/state;
- `Esc`: close the window.

The example reports all six shader artifacts at startup. The graphics shaders
(`world.vert`, `world.frag`, `ui.vert`, and `ui.frag`) are loaded by the native
presentation path. The compute shaders (`culling/frustum_cull.comp` and
`culling/build_indirect.comp`) are compiled and their Vulkan pipeline,
descriptor, barrier, and indirect-draw contracts are available in both PALs;
the current voxel presenter still uses the CPU extraction path until compute
buffer lifetime is enabled in the frame owner.

### GPU validation

GPU CI is intentionally disabled for now. Run the voxel example locally on a
machine with a Vulkan driver. Linux and Windows both contain the native GPU
pipeline contracts, but hardware presentation remains dependent on the local
loader, driver, window system, and device.
