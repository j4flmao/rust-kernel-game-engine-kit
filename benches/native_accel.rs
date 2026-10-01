//! Checked scalar Rust versus the backend actually selected on this host.
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rust_kernel_game_engine_kit::native_accel::{self as selected, reference};
use std::time::Duration;

const SIZES: [usize; 4] = [1_000, 64_000, 1_000_000, 1_000_003];
const PLANES: [f32; 24] = [
    1., 0., 0., 1., -1., 0., 0., 1., 0., 1., 0., 1., 0., -1., 0., 1., 0., 0., 1., 1., 0., 0., -1.,
    1.,
];

fn bench_native(c: &mut Criterion) {
    eprintln!(
        "native benchmark backend={} arch={} os={}",
        selected::backend_name(),
        std::env::consts::ARCH,
        std::env::consts::OS
    );
    let mut group = c.benchmark_group("native_accel");
    group.sample_size(10);
    group.warm_up_time(Duration::from_millis(100));
    group.measurement_time(Duration::from_secs(1));
    for count in SIZES {
        let positions: Vec<_> = (0..count * 3)
            .map(|i| ((i * 17 % 1021) as f32 - 510.) / 100.)
            .collect();
        let indices: Vec<_> = (0..(count / 3 * 3) as u32).collect();
        let matrix = [1.1, 0.2, 0., 3., 0., 0.9, 0.1, -2., 0., 0., 1.2, 0.5];
        group.throughput(Throughput::Elements(count as u64));
        for (label, scalar) in [("rust-reference", true), (selected::backend_name(), false)] {
            let mut output = vec![0.; positions.len()];
            group.bench_function(BenchmarkId::new(format!("transform/{label}"), count), |b| {
                b.iter(|| {
                    let run = if scalar {
                        reference::batch_transform_xyz
                    } else {
                        selected::batch_transform_xyz
                    };
                    run(
                        black_box(&positions),
                        black_box(&mut output),
                        black_box(&matrix),
                    );
                    black_box(&output);
                });
            });
            group.bench_function(BenchmarkId::new(format!("normals/{label}"), count), |b| {
                b.iter(|| {
                    let run = if scalar {
                        reference::generate_vertex_normals
                    } else {
                        selected::generate_vertex_normals
                    };
                    run(
                        black_box(&positions),
                        black_box(&indices),
                        black_box(&mut output),
                    );
                    black_box(&output);
                });
            });
            group.bench_function(BenchmarkId::new(format!("aabb/{label}"), count), |b| {
                b.iter(|| {
                    let run = if scalar {
                        reference::calculate_aabb
                    } else {
                        selected::calculate_aabb
                    };
                    black_box(run(black_box(&positions)))
                });
            });
            for percent in [0, 50, 100] {
                let spheres: Vec<_> = (0..count)
                    .flat_map(|i| [if i % 100 < percent { 0. } else { 3. }, 0., 0., 0.25])
                    .collect();
                let mut visible = vec![0; count];
                group.bench_function(
                    BenchmarkId::new(format!("cull-{percent}pct/{label}"), count),
                    |b| {
                        b.iter(|| {
                            let run = if scalar {
                                reference::batch_cull_spheres
                            } else {
                                selected::batch_cull_spheres
                            };
                            run(
                                black_box(&spheres),
                                black_box(&PLANES),
                                black_box(&mut visible),
                            );
                            black_box(&visible);
                        });
                    },
                );
            }
        }
    }
    group.finish();
}

criterion_group!(native_accel_benches, bench_native);
criterion_main!(native_accel_benches);
