# Benchmarks and reports

## Two suites

`benches/kernel.rs` measures kernel/runtime primitives. `benches/quality_milestones/performance_milestone.rs` measures the separate milestone harness. Keep their output directories separate when comparing results.

```powershell
cargo bench --locked --bench kernel
$env:CARGO_TARGET_DIR = "target/criterion-milestone"
cargo bench --locked --bench performance_milestone
Remove-Item Env:CARGO_TARGET_DIR
```

With the default target directory, the kernel HTML entry point is `target/criterion/report/index.html`. The milestone equivalent is `target/criterion-milestone/criterion/report/index.html`.

## Milestone cases

| Group | Cases |
| --- | --- |
| Object scale | 1M, 4M, 9M, 18M objects; one partition; 60 FPS target |
| Partition scale | 18M objects; 1, 4, 16, 64, 256, 1024 partitions |
| Cadence targets | 18M objects; 64 partitions; 60 and 120 FPS |

The checked-in suite contains 12 parameter combinations. Each uses 10 Criterion samples and a one-second requested measurement interval. The harness accepts larger `u64` counts subject to validation; extending the benchmark matrix requires adding configurations to the benchmark source.

::callout{type="warning"}
18M is a workload target, not a measured claim that the engine renders 18M visible objects at 60 FPS. These cases time CPU harness work. A 120 FPS label specifies a cadence budget; it does not turn a CPU benchmark into a GPU test.
::

## Interpreting a report

The harness partitions a sequential CPU loop. It does not spawn one worker per partition and does not allocate an ECS entity for each object. The loop updates a checksum from object IDs; changing the FPS target changes the budget, not the arithmetic workload. Read `harness.rs::run_frame` before applying these results to actual rendering.

## Local worked result — 2026-09-28

Both suites completed on Windows with `rustc 1.95.0 (59807616e 2026-04-14)`. This was a documentation smoke measurement: the runs were launched together, compilation and background work overlapped, and the machine was not isolated. These numbers illustrate report interpretation, not a performance baseline.

| Criterion case | Displayed point estimate | Confidence interval |
| --- | --- | --- |
| kernel: memory/arena/64 | 39.983 ns | 38.857–40.870 ns |
| kernel: ui/layout-paint/512 | 240.89 µs | 237.86–247.66 µs |
| milestone: 18M, 1 partition, object scale | 16.124 ms | 12.787–21.909 ms |
| milestone: 18M, 64 partitions, 60 FPS cadence | 10.025 ms | 9.9694–10.113 ms |
| milestone: 18M, 64 partitions, 120 FPS cadence | 10.033 ms | 9.9414–10.174 ms |

A 60 FPS frame budget is approximately 16.667 ms; 120 FPS allows 8.333 ms. The latter CPU preparation measurement alone exceeds the 120 FPS budget. Even a mean below a budget cannot prove every frame meets it, and rendering/input/simulation costs are not included here.

The arena case measures a batch of 64 allocations followed by iteration bookkeeping, not one allocation. The confidence interval concerns Criterion's estimate; it is not a p95/p99 frame-time interval. Existing-baseline “improved” messages cannot establish a regression result without controlling machine load and run settings.

### Commands used for this smoke measurement

```powershell
cargo bench --locked --bench kernel -- --sample-size 10 --warm-up-time 0.1 --measurement-time 0.2
$env:CARGO_TARGET_DIR = "target/criterion-milestone"
cargo bench --locked --bench performance_milestone -- --warm-up-time 0.1
Remove-Item Env:CARGO_TARGET_DIR
```

The milestone source overrides sample count and measurement time per group (10 samples, one second). For a baseline run, execute suites sequentially on an otherwise idle host with longer warmup and retain raw `new/estimates.json`, `new/sample.json`, and hardware metadata alongside the HTML report.

### Open the generated HTML locally

```powershell
Start-Process target/criterion/report/index.html
Start-Process target/criterion-milestone/criterion/report/index.html
```

These paths are relative to the repository root. Keep the complete Criterion directory when sharing the report: copying only `index.html` loses charts and linked case pages.

Compare identical workload IDs, build profiles, and hardware. Record toolchain, CPU, OS, and configuration. Hosted runner variance makes single-run performance claims unreliable. Inspect confidence intervals and distributions before attributing small changes to a patch.

## CI artifacts

The quality milestone workflow uploads `quality-milestone-criterion-report`. Download and extract the whole artifact so the HTML can load its supporting files. The workflow also checks that HTML was produced. Missing baseline files should be investigated as report/cache state rather than assumed to be a renderer fault.

Source: [milestone benchmark](https://github.com/j4flmao/rust-kernel-game-engine-kit/blob/main/benches/quality_milestones/performance_milestone.rs).
