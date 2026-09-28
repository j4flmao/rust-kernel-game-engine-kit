# Quality milestone benchmarks

These Criterion workloads are separate from `benches/kernel.rs`. They measure
object preparation, partition/batch scaling, and 60/120 FPS cadence targets.

```text
cargo test --test performance_milestone --release
CARGO_TARGET_DIR=target/criterion-milestone cargo bench --bench performance_milestone
```

The HTML report is written below `target/criterion-milestone/criterion/`, so it
does not mix with the kernel report in `target/criterion/`.

The 18M object count is the default milestone, not a hard maximum. The shared
harness accepts larger `u64` object counts and rejects arithmetic overflow
before entering the hot loop.
