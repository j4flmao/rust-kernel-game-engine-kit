# Delivery Roadmap

This roadmap follows the repository's priority order: security, stability, then speed.

## Phase 0 — Documentation and contracts

Status: **current / this document set**

- Freeze architecture terminology.
- Document current versus modeled versus planned behavior.
- Define acceptance tests before adding platform-specific code.
- Keep the headless build as the baseline.

Exit criteria:

- Docs link cleanly.
- Every planned subsystem has an owner boundary, failure policy, and test class.
- No document claims native support without a host validation command.

## Phase 1 — Runtime schedule graph

Status: **partially implemented**

- Add explicit system access metadata.
- Build a persistent schedule graph from system sets and access conflicts.
- Execute independent nodes in bounded dependency waves.
- Add deferred structural command buffers and commit points.
- Keep the existing Subsystem API as a compatibility adapter during migration.

Implemented slice: subsystems now expose `SystemAccess`, kernel initialization builds a
validated `SchedulePlan`, and access-aware waves are observable through the kernel. The
kernel now also exposes a bounded deferred command buffer committed after each frame.
The tick callback path is intentionally still serial until isolated contexts are
implemented.

Exit criteria:

- Cycle and missing-dependency diagnostics are deterministic.
- Read/read systems can run concurrently.
- Conflicting writes never overlap.
- A serial reference executor and parallel executor produce equivalent observable results.
- No unbounded task creation or hidden allocation in the tick hot path.

## Phase 2 — Render extraction and render world

Status: **partially implemented**

- Add an immutable extracted render snapshot.
- Add stable instance IDs, change ticks, visibility flags, and bounded extraction.
- Separate simulation lifetime from GPU resource lifetime.
- Add CPU fallback sorting and batching before GPU-driven mode.

Implemented slice: `RenderWorld` provides bounded frame-owned instances and typed ECS
projection, while `CpuRenderManifest` provides deterministic mesh/material batching.
`GpuFramePreparation` and `GpuFrameUpload` now validate bounded instance uploads and
indirect command ranges on the CPU reference path. The `RendererSubsystem` exposes the
same upload entry point for a native composition root, producing stable instance-record
and exact Vulkan indirect-command bytes without borrowing the ECS world. Native Vulkan
compute culling remains planned; native world-pass presentation exists as a bring-up path. A `RenderBackend` contract and
deterministic headless backend now validate the prepared frame without requiring a display
or GPU.

ECS lifetime hardening is also in place: despawn removes components from every
erased storage, stale handles cannot insert new components, and generation reuse
is covered by regression tests and the bounded ECS mutation fuzz target.

Exit criteria:

- Simulation and render preparation can use separate worlds.
- Resize, despawn, asset eviction, and frame reuse are race-safe.
- CPU and GPU paths produce equivalent draw-visible results for a reference scene.

## Phase 3 — Native Vulkan composition

Status: **native bring-up implemented; production host validation remains open**

Native platform modules contain presentation and world-pass resources. Host validation, recovery and general mesh rendering remain separate acceptance criteria. See [3D status](3d.md).

- Negotiate instance/device extensions and queue families.
- Add Linux surface policy and Windows Win32 surface policy.
- Create swapchain, image views, per-frame command pools, semaphores, and fences.
- Wire vkQueueSubmit and present into the renderer subsystem.
- Keep headless mode available when no surface or suitable device exists.

Exit criteria:

- Windows native smoke test runs on a Windows host with a Vulkan driver.
- Native Linux smoke test runs on a native Linux host with the selected display backend.
- WSL validates Linux compilation/headless behavior only unless a configured display path is
  explicitly available.
- Swapchain recreation and device-loss paths are tested.

## Phase 4 — GPU-driven rendering

Status: **planned**

- Upload compact instance/material/mesh tables.
- Dispatch bounded compute culling and binning.
- Generate indirect draw commands.
- Insert explicit synchronization and validate all GPU-written counts.
- Add CPU fallback for unsupported features, small scenes, and debugging.

Exit criteria:

- GPU-driven and CPU fallback images or draw manifests agree for a fixed test scene.
- GPU buffer growth and overflow are handled without memory corruption.
- Frame-time, upload bytes, culling work, and draw count are benchmarked.

## Phase 5 — Streaming and replication

Status: **partially implemented; end-to-end hardening remains planned**

`src/kernel/sync.rs` already implements replication queues, authentication and file-backed WAL operations. Platform modules include io_uring and IOCP code. The tasks below are completion and integration criteria; they should not be read as an absence of these primitives.

- Implement io_uring SQ/CQ mapping and bounded async file operations on Linux.
- Implement IOCP overlapped file streaming on Windows.
- Connect streaming completion to asset state transitions and backpressure.
- Make WAL rotation/compaction and replication replay bounded and crash-safe.

Exit criteria:

- End-to-end tests cover completion, cancellation, short reads, errors, and shutdown.
- Queue depth and memory use remain bounded under sustained load.
- Recovery tests prove no duplicate or lost committed record after restart.

## Phase 6 — Reliability and security gates

Status: **partially current / expansion planned**

- Keep cargo audit, deny/warnings, Miri, ASan, fuzz, property tests, and benchmarks.
- Add bounded-input fuzz targets for schedule, ECS mutations, extraction, and serialization.
- Add long-running soak tests and failure-seed regression tests.
- Publish Criterion HTML reports and machine-readable summaries as artifacts.

Exit criteria:

- Fast PR gates are deterministic and fit the agreed timeout.
- Nightly gates are broader, not silently allowed to fail.
- A failing fuzz/property seed becomes a checked-in regression case.
- Security findings have an owner and a disposition.

Current fuzz coverage includes schedule/access-wave inputs, ECS mutations, and
render extraction/preparation. Serialization fuzzing and native sanitizer runs
remain CI/Linux work.

## Phase 7 — Optional no_std track

Status: **separate smoke crate implemented; broader extraction remains planned**

`kernel_nostd/lib.rs` provides a `no_std` smoke library with a 64 KiB bump allocator, allocation statistics, a spin-loop panic handler and a smoke export. Its allocator does not reclaim memory. CI checks `kernel_nostd/Cargo.toml` for `wasm32-unknown-unknown`. This is not a no_std build of the full engine.

- Extract platform-independent contracts and bounded data structures.
- Add a no_std library target with an explicit allocator/panic policy.
- Keep OS threads, files, Vulkan, and networking outside the no_std crate.
- Validate wasm32/no_std first; do not imply bare-metal support without a target HAL.

Exit criteria:

- no_std builds without accidental std dependencies.
- Allocation failure is explicit.
- Panic and synchronization policy is documented per target.
