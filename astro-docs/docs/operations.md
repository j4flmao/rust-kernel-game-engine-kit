# Configuration and troubleshooting

## Environment variables

| Variable | Consumer | Behavior |
| --- | --- | --- |
| `RUST_KERNEL_NATIVE_VULKAN` | Main binary, Windows | Presence enables native bootstrap attempt; even a value of `0` is present |
| `RKE_ENABLE_NATIVE_VULKAN` | Sudoku, Windows | `experimental` opts into its Vulkan attempt |
| `RKE_DISABLE_GDI` | Sudoku native bridge | Presence disables its GDI path |
| `SUDOKU_DIFFICULTY` | Sudoku model | `medium` or `hard`; see model default for other values |
| `RKE_RUBIK_MAX_FRAMES` | Rubik | Parsed frame limit; default 600; zero ends after initialization |
| `CARGO_TARGET_DIR` | Cargo | Selects build/report location; restore the previous value after temporary overrides |
| `DOCS_BASE` | Astro build | URL prefix; default `/` |

These switches belong to different composition roots. Setting the main binary's flag does not select Sudoku's backend.

## Native rendering failure

First reproduce headless contract tests. Then record the OS, adapter, Vulkan driver, exact example command and stderr. A successful Windows example build checks compilation, not successful presentation. For missing/black faces, distinguish scene transforms, depth/culling state and shader data from window startup problems.

## Clicks and resizing

Compare client dimensions, DPI, layout rectangles and mouse coordinates. After a modal or scene transition, inspect focus/capture and active input state. See [UI guide](ui-guide.md). Rebuilding the window on every edit can create flicker without fixing the state issue.

## Queue and memory failures

Inspect capacity errors at the owner boundary: bus inbox, deferred queue, UI nodes, render instances or replication payloads. Measure current and peak usage before raising limits. A larger queue cannot repair a consumer that never drains.

## Tracing and frame timing

`Kernel::run_until` records each callback's duration in `FrameTracer` using `TraceSample`. The default callback debug budget is 1 ms; it is distinct from the 60 Hz frame period. Trace code lives in `src/kernel/trace/`, with regression coverage in `tests/kernel_trace.rs`. The `race_trace` feature enables additional contention instrumentation; it is not a data-race proof.

## no_std smoke target

```sh
rustup target add wasm32-unknown-unknown
cargo check --manifest-path kernel_nostd/Cargo.toml --target wasm32-unknown-unknown
```

This checks the separate smoke crate, not the std engine or its native renderer. It uses a bounded allocator and a spin-loop panic policy; there is no demonstrated bare-metal game runtime implied by this command.
