# Source and test map

Use this map to find the implementation behind a guide. The architectural plans describe intended evolution; source and executable tests establish present behavior.

| Concern | Implementation | Starting verification |
| --- | --- | --- |
| Composition | `src/main.rs` | `cargo run --release` |
| Driver contract | `src/kernel/subsystem.rs`, `game.rs` | `tests/kernel_scheduler.rs` |
| Scheduling and deferred work | `src/kernel/scheduler.rs`, `deferred.rs` | `tests/kernel_scheduler.rs`, `kernel_world.rs` |
| Message routing | `src/kernel/bus.rs`, `context.rs` | `tests/kernel_bus.rs`, `subsystem_bus_integration.rs` |
| Entities and storage | `src/kernel/ecs/` | `tests/kernel_world.rs` |
| Memory | `src/kernel/mem/` | Inline unit tests; kernel Criterion suite |
| Replication and WAL | `src/kernel/sync.rs` | Inline unit tests; property harness |
| Frame telemetry | `src/kernel/trace/` | `tests/kernel_trace.rs` |
| Platform clock / OS bindings | `src/platform/` | `tests/linux_time.rs`, `linux_syscalls.rs`; host-gated tests |
| Rendering packets | `src/subsystems/renderer.rs`, `renderer_3d.rs` | Inline contract tests |
| UI layout/input/paint | `src/subsystems/ui/` | `tests/ui_rendering.rs` |
| Gameplay examples | `examples/sudoku_game/`, `examples/rubik_3d/` | Example unit tests and manual native runs |
| Milestone preparation | `benches/quality_milestones/` | `tests/performance_milestone.rs` |

## Focused commands

```sh
cargo test --locked --test kernel_scheduler
cargo test --locked --test kernel_bus
cargo test --locked --test kernel_world
cargo test --locked --test ui_rendering
cargo test --locked --example rubik_3d --release
cargo test --locked --test performance_milestone --release
cargo doc --no-deps --open
```

`cargo doc` generates the symbol-level Rust API reference. Use this site for concepts and worked examples, and rustdoc for the complete signature surface. OS-specific tests need their corresponding host; a skipped native capability is not a successful native rendering run.

## Review a change end to end

Find the owner module, read its error and capacity contract, locate its unit/integration tests, and reproduce a focused command. For performance changes, run the matching Criterion group after correctness checks. For native changes, additionally capture device and driver details on an appropriate host.
