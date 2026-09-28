# Rust Kernel Engine

A Rust game-engine foundation built around explicit subsystem contracts. The kernel coordinates lifecycle, dependency ordering, messages, ECS state, bounded memory, and tracing. Platform and game behavior live behind those contracts.

## Start reading

- [Getting started](getting-started.md): build the engine and run examples.
- [Kernel lifecycle](runtime.md): registration, ticks, messages, and shutdown.
- [Architecture](architecture.md): ownership boundaries and extension points.
- [Benchmarks](benchmarks.md): kernel reports and quality milestones.

## Implementation status

| Area | What exists | Qualification |
| --- | --- | --- |
| Kernel | Dependency validation, fixed-step callbacks, schedule waves | Subsystem tick callbacks remain serial |
| ECS | Generational entities, component storage, queries | Check individual APIs for mutation and capacity rules |
| Rendering | Extraction, CPU batching, GPU upload records, headless backend | Native presentation requires platform capability |
| Examples | Sudoku UI and a Rubik 3D demonstration | Windows is the documented interactive path |
| Performance | Criterion kernel and milestone suites | CPU measurements do not establish GPU frame rate |

## Reading status labels

**Current** means an implementation exists. **Modeled** means a contract or isolated primitive exists without complete runtime integration. **Planned** describes future work. **Blocked** requires a capability such as a native GPU host. Design references retain their own acceptance criteria; a roadmap item is not a runtime guarantee.

## Dependencies and scope

The engine uses handwritten platform bindings, but the current package is not dependency-free: `Cargo.toml` declares `hmac`, `sha2`, and `libc`, with Criterion as a development dependency. The main library uses `std`; any separate no_std track must be evaluated independently.

Source: [Cargo.toml](https://github.com/j4flmao/rust-kernel-game-engine-kit/blob/main/Cargo.toml), [kernel](https://github.com/j4flmao/rust-kernel-game-engine-kit/tree/main/src/kernel).
