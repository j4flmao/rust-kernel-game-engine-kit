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
height-mapped block terrain, removes hidden faces (including chunk seams), and
greedily merges coplanar faces with the same material inside 16×16×16 chunks.
It opens the native Vulkan presentation path on Windows or Linux/X11. The scene is intentionally
separate from the Rubik demo and is used to stress cube geometry, instance
uploads, depth testing, player movement, camera movement, and frame pacing.

Mesh generation is cached by seed and block budget, not rebuilt for camera
movement. Six directional draw batches use the existing 80-byte instance ABI;
terrain uses a separate shader material flag, not Rubik facelet colors.
By default terrain is finite. `RKE_GPU_STRESS_BLOCKS`
is an approximate solid-block budget (clamped to 1–250,000), not a draw count.
Uploads are capped at 4 MiB on both example paths. Console counters distinguish
solid blocks, exposed faces, greedy quads, draw calls, and upload bytes.

### Bounded chunk streaming

Set `RKE_GPU_STRESS_STREAM_RADIUS` to `1`–`4` to stream procedural terrain
around the player; `0` (default) preserves the finite benchmark. Radius `r`
keeps at most `(2*r+1)^2` chunk columns (81 at radius 4), each 16×16 blocks
wide and 16 blocks high. In streaming mode the radius replaces the block-count
budget; actual solid-block counts are reported in the console.

```powershell
$env:RKE_GPU_STRESS_STREAM_RADIUS = "4"
$env:RKE_GPU_STRESS_MAX_FRAMES = "3600"
cargo run --release --example gpu_stress_3d
```

```sh
RKE_GPU_STRESS_STREAM_RADIUS=4 RKE_GPU_STRESS_MAX_FRAMES=3600 \
  cargo run --release --example gpu_stress_3d
```

The streamer builds at most two chunks per frame, prioritizes nearby chunks,
evicts out-of-range chunks, and reuses retained meshes. Neighbor queries use the
global terrain generator, including negative coordinates, so loaded chunk seams
do not produce internal faces. `H` invalidates the resident set and regenerates
it incrementally. Returning to an evicted region reconstructs the same terrain.
The title/console expose resident and pending chunk counts.

This is synchronous, bounded streaming, not an infinite Minecraft world:
player X/Z coordinates are clamped near ±65,000 for f32 precision. Far chunks
appear progressively, without skirts or loading-screen masking. Residency and
the assembled upload each have a 4 MiB geometry cap; exceeding it is an explicit
error. Each residency change repacks and uploads the full active mesh (at most
once per frame), not GPU subrange updates. Once the queue is drained, stationary
terrain causes no more transfers. Persistent edits, background meshing, LOD,
GPU compute culling, and chunk-local GPU allocation remain future work.

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

Linux requires an X11 display (or XWayland), a Vulkan driver, and `glslc` on PATH:

```sh
RKE_REQUIRE_GLSLC=1 RKE_GPU_STRESS_BLOCKS=100000 RKE_GPU_STRESS_MAX_FRAMES=600 \
  cargo run --release --example gpu_stress_3d
```

Headless meshing tests, usable on both platforms:

```sh
cargo test --example gpu_stress_3d
```

Controls:

- left drag: orbit the world camera;
- `W/A/S/D`: move along world axes; `Space`: jump;
- `H`: regenerate the stress scene;
- `N`: reset the camera/state;
- `Esc`: close the window.

The example reports all six shader artifacts at startup; this inventory is not
proof that all six execute. Terrain draws use `world.vert` and `world.frag`.
The help overlay currently uses Windows GDI; Linux controls are documented here.
The compute shaders (`culling/frustum_cull.comp` and
`culling/build_indirect.comp`) are compiled and their Vulkan pipeline,
descriptor, barrier, and indirect-draw contracts are available in both PALs;
the current voxel presenter still uses CPU greedy meshing until compute
buffer lifetime is enabled in the frame owner.

The presenter serializes shared upload/depth resources for correctness. Geometry
uploads only after a mesh change; camera-only frames update push constants without
copying mesh bytes. Resize keeps the resident mesh buffer. Failed acquisition
does not consume a pending upload; counters advance only after successful submit.
Both backends reject partial 80-byte records and out-of-range instance draws.

The window title reports FPS, blocks, quads, draws, and cumulative geometry
transfers/bytes. The console prints final transfer counters. Per-frame GPU
resource rings and GPU timestamp queries remain future work.

Set `RKE_GPU_STRESS_TARGET_FPS=60` (default), `120`, or `0` to disable CPU pacing.
Pacing subtracts frame work from the sleep budget. Vulkan presentation may still
synchronize to the display. These are requested caps, not performance guarantees;
reported FPS includes CPU work, presentation, and the Windows help overlay.

```powershell
$env:RKE_GPU_STRESS_TARGET_FPS = "120"
cargo run --release --example gpu_stress_3d
```

```sh
RKE_GPU_STRESS_TARGET_FPS=0 cargo run --release --example gpu_stress_3d
```

### GPU validation

GPU CI is intentionally disabled for now. Run the voxel example locally on a
machine with a Vulkan driver. Linux and Windows both contain the native GPU
pipeline contracts, but hardware presentation remains dependent on the local
loader, driver, window system, and device.
