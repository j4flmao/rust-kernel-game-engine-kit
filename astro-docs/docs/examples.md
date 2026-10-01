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

### Rubik controls

| Input | Action |
| --- | --- |
| Left drag on a sticker | Request a layer turn |
| Right drag | Orbit the whole cube |
| U, D, L, R, F, B | Request the corresponding face move |
| M, E, S | Request a middle-slice move |
| H | Queue a 24-move shuffle |
| Z / Y | Undo / redo |
| T | Solve by reversing recorded history |
| N | Reset the cube |
| 0 | Reset the camera |
| Escape | Close the session |

The history solver reverses recorded moves; it is not a general solver for an arbitrary imported cube. Cubie-local sticker colors stay attached to each cubie while its coordinate and orientation change. Animation interpolates the active layer before committing the discrete state.

Source: [Rubik input loop](https://github.com/j4flmao/rust-kernel-game-engine-kit/blob/main/examples/rubik_3d.rs).

### Hardware coverage

GPU CI is intentionally disabled for now. The native Rubik example can still
be run locally on a machine with a Vulkan driver. A dedicated GPU workflow will
be added after a GPU server is available; it will then cover adapter discovery,
native Vulkan presentation, GPU frame timing, and the 18M/60/120 milestones.

## `gpu_stress_3d`

`gpu_stress_3d` is a separate Minecraft-style voxel renderer; it does not reuse
Rubik state or layer-turn logic. It generates height-mapped block terrain,
removes hidden faces across chunk boundaries, greedily merges same-material
surfaces inside 16×16×16 chunks, and caches the mesh until its seed or budget changes.
It presents through native Vulkan on Windows and Linux/X11. The player state has gravity, jumping, and bounded WASD
movement, while the orbit camera follows the player.

```powershell
$env:RKE_GPU_STRESS_BLOCKS = "16384"
$env:RKE_GPU_STRESS_MAX_FRAMES = "600"
cargo run --release --example gpu_stress_3d
```

Controls:

| Input | Action |
| --- | --- |
| W / A / S / D | Move the player |
| Space | Jump |
| Mouse drag | Orbit the camera around the player |
| H | Regenerate terrain seed |
| N | Reset player and camera |
| Escape | Close the window |

The example prints all six generated shader artifacts at startup; inventory does
not imply execution. Terrain uses world graphics shaders with a dedicated solid
material flag; Windows help uses GDI. The compute
shader modules and matching Linux/Windows Vulkan descriptor, pipeline, barrier,
dispatch, and indirect-draw contracts are implemented, but live compute
buffer ownership remains a follow-up before the example switches from CPU
meshing to GPU culling.

Linux requires X11/XWayland, a Vulkan driver and `glslc`:

```sh
RKE_REQUIRE_GLSLC=1 RKE_GPU_STRESS_BLOCKS=100000 RKE_GPU_STRESS_MAX_FRAMES=600 \
  cargo run --release --example gpu_stress_3d
```

The default scene is finite terrain. The block setting is
approximate. Console counters distinguish actual blocks, exposed faces, greedy
quads, draw calls and upload bytes. The upload cap is 4 MiB. Geometry uploads only
when the mesh changes; camera frames update push constants. Shared GPU buffers
and depth are serialized for correctness. Window-title statistics and final
console counters distinguish resident mesh size from cumulative transferred bytes.

`RKE_GPU_STRESS_TARGET_FPS` selects a CPU pacing cap: `60` (default), `120`, or `0`
for no CPU pacing. Presentation can still synchronize to the display. FPS includes
CPU work, presentation and any help overlay, not isolated GPU timing.

`cargo test --example gpu_stress_3d` checks material boundaries, cache reuse,
upload bounds, and surface area against a brute-force face reference.

### Streaming mode

Set `RKE_GPU_STRESS_STREAM_RADIUS=1`–`4` (default `0` disables streaming).
It replaces the finite block budget with a resident square of `(2*r+1)^2`
16×16×16 chunk columns, at most 81 chunks. Only two missing chunks are meshed
per frame, nearest first. Out-of-range chunks are evicted and retained chunks
are reused. The title and console show resident/pending counts.

```powershell
$env:RKE_GPU_STRESS_STREAM_RADIUS = "4"
$env:RKE_GPU_STRESS_MAX_FRAMES = "3600"
cargo run --release --example gpu_stress_3d
```

```sh
RKE_GPU_STRESS_STREAM_RADIUS=4 RKE_GPU_STRESS_MAX_FRAMES=3600 \
  cargo run --release --example gpu_stress_3d
```

Neighbor sampling crosses chunk boundaries and handles negative coordinates.
`H` rebuilds the resident set incrementally; returning to an evicted area is
deterministic. World X/Z positions are bounded near ±65,000 for f32 precision.
Chunk arrival can be visible: there are no terrain skirts or background jobs.

The 4 MiB residency/upload bounds remain in force. Each residency change repacks
the complete active mesh into six directional batches and uploads it once;
GPU subrange updates, persistent edits, LOD and live compute culling are not
implemented. This is bounded streaming, not an infinite-world implementation.
Tests cover work budgets, eviction, negative boundaries, teleports, regeneration,
returning to an area, and seam faces against global terrain sampling.
