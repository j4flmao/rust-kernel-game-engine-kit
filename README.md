# rust-kernel-game-engine-kit

rust-kernel-game-engine-kit is a **microkernel-style game-engine foundation**. The kernel owns subsystem lifecycle, dependency ordering, scheduling, messaging, ECS, memory primitives, synchronization, and telemetry. Rendering, windowing, input, audio, physics, scripting, I/O, and platform backends stay outside the kernel and communicate through explicit contracts.

The project follows this priority order:

1. **Security** — bound external input, validate state transitions, authenticate wire packets, and prevent unbounded allocation.
2. **Stability** — return initialization/runtime failures through `Result`, reject dependency cycles, and bound WAL/queue growth.
3. **Performance** — use fixed timesteps, arenas, pools, ring buffers, worker pools, and dependency waves only behind validated boundaries.

> Current status: this repository provides a headless engine path plus native PAL/Vulkan/I/O primitives. It is not a claim that every native driver has been validated on every GPU, driver, or operating-system configuration. Target hardware and CI remain the final authority.

## Features

### Kernel and scheduling

- `Kernel` manages subsystem registration, `init`, fixed-timestep `run`, and `shutdown`.
- The dependency graph is topologically sorted; missing dependencies and cycles fail startup.
- `dependency_waves` groups independent nodes into deterministic execution waves.
- `ParallelWaveExecutor` executes independent jobs on a bounded thread pool.
- `KernelContext` is the boundary for the clock, message bus, and ECS world. Subsystems do not need direct access to one another.
- Frame tracing and contention tracing are available through the optional `race_trace` feature.

### Message bus and ECS

- Bounded SPSC/MPSC ring queues return an error when full instead of silently growing memory.
- Subscriber topology and debug edges validate sender/receiver relationships.
- The hand-rolled ECS includes entity generations, sparse component storage, bitsets, and queries.
- Generation counters detect stale entity and pool handles after slot reuse.
- Example subsystems include window, input, renderer, physics, audio, scripting, hello, and tock.

### Memory and overflow policy

- `BumpArena` provides per-frame scratch memory; `Pool<T>` handles long-lived high-churn objects; `SpscRing` supports streaming data.
- `try_*` constructors use fallible reservation and checked arithmetic so production boundaries can handle OOM explicitly.
- Convenience constructors such as `new` and `with_capacity` may panic on allocation failure; use `try_*` at production boundaries.
- Arena reset does not drop objects written into the region. Use arenas only for trivially destructible data, or drain owned values before reset.
- Packet, queue, topology, WAL-record, and buffer sizes are bounded by configuration.

### Synchronization, WAL, and security

- `Replicator` supports snapshots, deltas, events, acknowledgements, pending/in-flight bounds, and offline-first operation.
- `SyncWal` provides bounded replay, record limits, payload limits, and compaction to prevent unbounded file growth.
- `HmacSha256Authenticator` uses HMAC-SHA-256 to authenticate packet integrity and origin.
- Session nonces are checked to reject packets from another session.
- HMAC **does not encrypt data**. If confidentiality is required, use a standard AEAD transport/session layer; do not build encryption from HMAC.
- Keys must not be hard-coded or committed. Load them from a secret manager or OS credential store.

### Platform, Vulkan, and I/O

- The PAL provides a common monotonic clock and platform selection boundary.
- Linux includes raw clock/syscall helpers, dynamic loading, X11 surface support, Vulkan loader/device helpers, swapchain support, io_uring SQ/CQ mapping, and futex-oriented threading primitives.
- Windows includes Win32 window/surface support, Vulkan loader/device helpers, swapchain support, dynamic loading, CPU/threading helpers, and IOCP overlapped file streaming.
- The native renderer models command-buffer, semaphore, fence, acquire, submit, and present lifecycle validation.
- `examples/gpu_stress_3d` is a standalone Minecraft-style voxel renderer. It
  generates height-mapped blocks, supports WASD movement, jumping, mouse orbit,
  terrain regeneration, resize handling, and a native Vulkan presentation loop.
- The shader bundle contains graphics shaders (`world.vert`/`world.frag` and
  `ui.vert`/`ui.frag`) plus compute shaders for frustum culling and indirect
  command generation. Both Linux and Windows PALs expose matching compute
  pipeline, descriptor, barrier, dispatch, and indirect-draw contracts.
- The voxel example runs on Windows and Linux/X11, with cached chunk-local greedy
  meshing, hidden-face removal, and separate terrain materials. The default finite world
  reports solid blocks, exposed faces, merged quads, and upload bytes separately.
  Unchanged terrain stays GPU-resident; camera updates do not re-upload geometry.
  `RKE_GPU_STRESS_TARGET_FPS` selects 60/120 FPS pacing or `0` for no CPU cap,
  with live window-title counters (not GPU timestamp measurements).
- The current voxel presenter uses CPU meshing and GPU graphics upload. The
  compute shaders are compiled and validated, but their descriptor/buffer
  lifetime is not yet enabled in the live example frame owner.
- The composition root defaults to headless mode. On Windows, set `RUST_KERNEL_NATIVE_VULKAN=1` to try the native Win32/Vulkan bootstrap; unavailable native setup falls back to the headless renderer.
- Linux native window bootstrap is not enabled in `src/main.rs` yet. Linux PAL primitives exist, but composition-root wiring and real X11/Vulkan hardware validation are still required.

### no_std, fuzzing, and benchmarks

- `kernel_nostd/` is a minimal `no_std`/`wasm32` track with a bounded bump allocator, fallible allocation, and panic handler.
- `fuzz/` contains a libFuzzer target named `scheduler_input`.
- `benches/kernel.rs` uses Criterion with HTML reports for allocator, ring, message bus, ECS, sync wire/HMAC, and dependency-wave workloads. The runtime parallel executor remains covered by tests because repeated benchmark-time synchronization is not a stable measurement boundary on every host.
- `benches/quality_milestones/` is a separate Criterion suite for the rendering/runtime quality milestone. It reports multiple matrices, not only the 18M-object baseline: object scale (1M/4M/9M/18M), partition scale (1/4/16/64/256/1024 partitions at 18M), and cadence targets (60/120 FPS at 18M and 64 partitions). The harness accepts larger `u64` object counts for deeper local experiments, subject to overflow and partition validation.

## Repository layout

```text
src/
  kernel/              Kernel, scheduler, bus, ECS, memory, sync, tracing
  platform/            PAL, thread pool, Linux/Windows backends, Vulkan, I/O
  subsystems/          Window, input, renderer, physics, audio, scripting
  lib.rs               Library entry point
  main.rs              Composition root/demo executable
tests/                 Integration and property-oriented tests
benches/
  kernel.rs             Kernel Criterion benchmarks and report
  quality_milestones/   Separate Criterion quality-milestone harness and report
fuzz/                  Separate cargo-fuzz package
kernel_nostd/          Optional no_std wasm32 track
.github/workflows/     Format, test, clippy, audit, fuzz, Miri, and ASan CI
```

## Requirements

- Rust stable and Cargo.
- Windows: MSVC toolchain is recommended for native Win32/Vulkan work.
- WSL/Linux: Rust stable, build-essential, and the Vulkan/X11 runtime required by the selected native path. Headless tests do not require a display.
- Fuzzing: Rust nightly and `cargo-fuzz`.
- Miri/ASan: Rust nightly; these gates run in Linux CI.

## Build and run

### Windows native

Run from PowerShell at the repository root:

```powershell
cargo check --all-targets --all-features
cargo test --all-targets --all-features -- --test-threads=1
cargo run --bin rust-kernel-kit
$env:RUST_KERNEL_NATIVE_VULKAN = "1"
cargo run --bin rust-kernel-kit
```

`RUST_KERNEL_NATIVE_VULKAN=1` enables the experimental native path. A compatible Vulkan loader/driver and valid window environment are required. If native window/device/swapchain creation fails, the demo reports the error and falls back to headless rendering.

### WSL/Linux

Run these commands inside WSL, not in PowerShell:

```bash
cargo check --all-targets --all-features
cargo test --all-targets --all-features -- --test-threads=1
cargo clippy --all-targets --all-features -- -D warnings
cargo run --bin rust-kernel-kit
```

Linux syscall/time tests are selected by target configuration. To test native X11/Vulkan, provide a valid `DISPLAY`/Wayland-X11 bridge and install the matching Vulkan runtime. Headless CI does not prove that presentation succeeds on a real driver.

### Voxel GPU stress example

On Windows, run the independent Minecraft-style block renderer:

```powershell
$env:RKE_GPU_STRESS_BLOCKS = "16384"
$env:RKE_GPU_STRESS_MAX_FRAMES = "600"
cargo run --release --example gpu_stress_3d
```

Controls are `WASD` to move, `Space` to jump, mouse drag to orbit the camera,
`H` to regenerate terrain, `N` to reset the player/camera, and `Esc` to exit.
Set `RKE_GPU_STRESS_STREAM_RADIUS=1`–`4` for bounded procedural streaming instead
of the finite block-count scene. It retains up to 81 chunks, builds at most two
per frame, and evicts distant chunks. Changed residency still repacks/uploads the
whole active mesh within a 4 MiB budget; per-chunk GPU uploads are not implemented.

## Native performance path

Native correctness compares transform, culling, normals and AABB with checked
Rust baselines. Dedicated workflows cover GCC/Clang, Windows fallback,
instrumented C and bounded fuzzing. Criterion labels the actual backend and
includes 48 cases; Windows fallback is never reported as C.
See [native validation](native/README.md) for commands and input contracts.

The optional `native-accel` feature provides batch CPU transform and frustum
culling through a small C ABI. Linux builds use the C implementation; Windows
keeps the same safe API but uses the Rust fallback by default so MSVC builds
remain reproducible. The feature is never called once per object.

```powershell
cargo test --all-features
cargo bench --bench native_accel --features native-accel
```

For strict GLSL validation, install `glslc` and run:

```powershell
$env:RKE_REQUIRE_GLSLC="1"
cargo check --all-targets --all-features
```

The repository supports Linux and Windows platform paths, but native Vulkan
presentation still depends on the installed driver, loader, window system,
and hardware. Headless Rust tests do not claim that a real GPU presented a
frame.

## Quality gates

Run these checks before opening a pull request:

```bash
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo test --all-targets --all-features -- --test-threads=1
cargo clippy --all-targets --all-features -- -D warnings
cargo audit
```

Fuzzing and sanitizers use nightly Rust:

```bash
cargo install cargo-fuzz
cargo fuzz run --target x86_64-unknown-linux-gnu scheduler_input -- -runs=1000
cargo miri setup
cargo miri test --lib -- --test-threads=1
cargo test -Zbuild-std --lib --target x86_64-unknown-linux-gnu -- --test-threads=1
```

For the ASan command, use nightly and set `RUSTFLAGS=-Zsanitizer=address`. The workflow configures this step. The `security-audit` job audits both the root package and the `fuzz` package.

### HTML benchmark reports

The project keeps kernel benchmarks and quality-milestone benchmarks in separate
Criterion output directories. This keeps the reports comparable and prevents a
milestone run from changing the kernel report.

#### Kernel report

```bash
cargo bench --bench kernel -- --sample-size 10 --warm-up-time 1 --measurement-time 1
```

Criterion writes the kernel index report to
`target/criterion/report/index.html` and per-group reports below
`target/criterion/`. The CI `property-and-bench` job uploads the complete
`target/criterion/` directory as the `kernel-benchmark-report` artifact.

#### Quality-milestone report

Run the milestone suite with its own target directory:

```bash
export CARGO_TARGET_DIR=target/criterion-milestone
cargo bench --bench performance_milestone
```

On PowerShell:

```powershell
$env:CARGO_TARGET_DIR = "target/criterion-milestone"
cargo bench --bench performance_milestone
```

The milestone report is written to
`target/criterion-milestone/criterion/report/index.html`. The report contains
12 benchmark cases across these groups:

- `quality_milestone/object_scale`: 1M, 4M, 9M, and 18M objects at 60 FPS.
- `quality_milestone/partition_scale`: 18M objects split across 1, 4, 16,
  64, 256, and 1024 partitions at 60 FPS.
- `quality_milestone/cadence_targets`: 18M objects and 64 partitions at both
  60 FPS and 120 FPS.

The 18M value is the published baseline, not a hard ceiling. The benchmark
harness can be configured for larger object counts in local experiments and
rejects invalid partition, FPS, and arithmetic-overflow configurations before
the hot loop. The 60 FPS budget is about 16.667 ms per frame; 120 FPS is about
8.333 ms. These workloads currently measure deterministic CPU-side preparation
and partition coverage, so the HTML report is a performance signal rather than
a claim that every GPU/driver can sustain the target.

The `quality-milestone` workflow uploads the complete milestone report as the
`quality-milestone-criterion-report` artifact. It intentionally runs without
Criterion's `--noplot` flag so that the HTML report and its plots are retained.
It runs for relevant pull requests, pushes to `main`, and can also be started
manually with the Actions `workflow_dispatch` button. It has no scheduled cron
run.

## Native I/O safety notes

`io_uring` and IOCP expose unsafe platform APIs. Callers must keep buffers and request objects alive until completion, must not reuse memory while the OS owns its pointer, and must handle queue-full, short-read, cancellation, completion errors, and shutdown ordering.

Vulkan handles must also be destroyed in ownership order: command resources and frame synchronization must be released before dependent device/surface objects. Resize must go through the recreate path and handle `OUT_OF_DATE`/`SUBOPTIMAL`. A valid model-state transition is not proof that a real driver presented successfully.

## Synchronization integration guidance

A production integration should follow this outline:

1. Create `SyncConfig` with payload, pending, and in-flight limits that fit the memory budget.
2. Open `SyncWal::open_with_limits` on a filesystem location dedicated to save/sync data.
3. Create `HmacSha256Authenticator` from a secret held outside the process image when possible, and set a fresh session nonce.
4. Authenticate packets before sending; verify the nonce and tag before applying payload data to game state.
5. Acknowledge in order and compact the WAL according to a checkpoint/size policy.
6. When connectivity fails, keep pending work bounded and apply an explicit back-pressure/drop policy; never spawn unbounded tasks.

This is a synchronization transport primitive, not a complete gameplay conflict-resolution system. The game still needs authoritative-server, ordering, idempotency, version/vector, and merge policies for each data type.

## Known limitations and production checklist

Before production, validate these items on supported hardware and workloads:

- Linux composition root creates a real window/device/swapchain and presents frames.
- Windows IOCP end-to-end streaming tests pass on every supported Windows version.
- Vulkan loader/driver matrix, resize, minimized windows, device loss, and recovery.
- Long-running fuzzing, multi-hour soak tests, Miri, and ASan—not only successful compilation.
- Fragmentation, allocation failure, queue pressure, packet flood, and WAL-growth benchmarks.
- Secret rotation, replay windows, transport AEAD, and product key management.
- Crash recovery after process termination during WAL append or compaction.
- Profiling on real game workloads before enabling core affinity or latency priority.
- No `.env`, private keys, save data, generated target artifacts, or temporary files committed to Git.

## License

Apache License 2.0. See [LICENSE](LICENSE) and `Cargo.toml` for the current package metadata. See [CONTRIBUTING.md](CONTRIBUTING.md) for development rules and [SECURITY.md](SECURITY.md) for private vulnerability reporting.
