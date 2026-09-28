//! Criterion report for the engine quality milestone.
//!
//! This benchmark intentionally uses its own Criterion groups under
//! `quality_milestone/`. CI sets `CARGO_TARGET_DIR=target/criterion-milestone`
//! so its HTML output never mixes with the kernel benchmark report.

use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

#[path = "harness.rs"]
mod harness;

use harness::{Config, Report};

const MEASUREMENT_TIME: Duration = Duration::from_secs(1);
const SAMPLE_SIZE: usize = 10;

fn benchmark_config(objects: u64, partitions: u32, target_fps: u32) -> Config {
    Config {
        objects,
        frames: 1,
        partitions,
        target_fps,
        warmup: 0,
        strict: false,
    }
}

fn measure(c: &mut Criterion, group_name: &str, configs: &[(String, Config)]) {
    let mut group = c.benchmark_group(group_name);
    group.sample_size(SAMPLE_SIZE);
    group.measurement_time(MEASUREMENT_TIME);
    for (id, config) in configs {
        group.throughput(Throughput::Elements(config.objects));
        group.bench_with_input(
            BenchmarkId::new("frame_prepare", id),
            config,
            |b, config| {
                b.iter(|| {
                    let report: Report = harness::run(black_box(*config));
                    black_box((report.processed, report.checksum, report.average_frame()));
                });
            },
        );
    }
    group.finish();
}

fn bench_object_scale(c: &mut Criterion) {
    let configs = [1_000_000_u64, 4_000_000, 9_000_000, 18_000_000]
        .into_iter()
        .map(|objects| {
            (
                format!("{objects}_objects_1_partition_60fps"),
                benchmark_config(objects, 1, 60),
            )
        })
        .collect::<Vec<_>>();
    measure(c, "quality_milestone/object_scale", &configs);
}

fn bench_partition_scale(c: &mut Criterion) {
    let configs = [1_u32, 4, 16, 64, 256, 1024]
        .into_iter()
        .map(|partitions| {
            (
                format!("18m_objects_{partitions}_partitions_60fps"),
                benchmark_config(18_000_000, partitions, 60),
            )
        })
        .collect::<Vec<_>>();
    measure(c, "quality_milestone/partition_scale", &configs);
}

fn bench_cadence_targets(c: &mut Criterion) {
    let configs = [60_u32, 120]
        .into_iter()
        .map(|target_fps| {
            (
                format!("18m_objects_64_partitions_{target_fps}fps"),
                benchmark_config(18_000_000, 64, target_fps),
            )
        })
        .collect::<Vec<_>>();
    measure(c, "quality_milestone/cadence_targets", &configs);
}

criterion_group!(
    name = quality_milestone_benches;
    config = Criterion::default();
    targets = bench_object_scale, bench_partition_scale, bench_cadence_targets
);
criterion_main!(quality_milestone_benches);
