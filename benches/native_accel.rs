//! CPU baseline for the optional native batch transform path.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

use rust_kernel_game_engine_kit::native_accel;

fn rust_transform(input: &[f32], output: &mut [f32], matrix: &[f32; 12]) {
    for (source, target) in input
        .as_chunks::<3>()
        .0
        .iter()
        .zip(output.as_chunks_mut::<3>().0)
    {
        let x = source[0];
        let y = source[1];
        let z = source[2];
        target[0] = matrix[0] * x + matrix[1] * y + matrix[2] * z + matrix[3];
        target[1] = matrix[4] * x + matrix[5] * y + matrix[6] * z + matrix[7];
        target[2] = matrix[8] * x + matrix[9] * y + matrix[10] * z + matrix[11];
    }
}

fn bench_transform(c: &mut Criterion) {
    let matrix = [1.1, 0.2, 0.0, 3.0, 0.0, 0.9, 0.1, -2.0, 0.0, 0.0, 1.2, 0.5];
    let mut group = c.benchmark_group("native_accel/batch_transform_xyz");
    for points in [1_000_usize, 64_000, 1_000_000] {
        let input = (0..points * 3)
            .map(|index| (index as f32).mul_add(0.001, -1.0))
            .collect::<Vec<_>>();
        group.throughput(Throughput::Elements(points as u64));
        group.bench_with_input(BenchmarkId::new("rust", points), &input, |b, input| {
            let mut output = vec![0.0; input.len()];
            b.iter(|| rust_transform(black_box(input), black_box(&mut output), black_box(&matrix)));
        });
        #[cfg(feature = "native-accel")]
        group.bench_with_input(BenchmarkId::new("c", points), &input, |b, input| {
            let mut output = vec![0.0; input.len()];
            b.iter(|| {
                native_accel::batch_transform_xyz(
                    black_box(input),
                    black_box(&mut output),
                    black_box(&matrix),
                )
            });
        });
    }
    group.finish();
}

fn bench_culling(c: &mut Criterion) {
    let planes = [
        1.0, 0.0, 0.0, 1.0, -1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, -1.0, 0.0, 1.0, 0.0, 0.0,
        1.0, 1.0, 0.0, 0.0, -1.0, 1.0,
    ];
    let mut group = c.benchmark_group("native_accel/batch_cull_spheres");
    for count in [1_000_usize, 64_000, 1_000_000] {
        let spheres = (0..count)
            .flat_map(|index| [((index % 32) as f32) - 16.0, 0.0, 0.0, 0.25])
            .collect::<Vec<_>>();
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(BenchmarkId::new("batch", count), &spheres, |b, spheres| {
            let mut visible = vec![0; count];
            b.iter(|| {
                native_accel::batch_cull_spheres(
                    black_box(spheres),
                    black_box(&planes),
                    black_box(&mut visible),
                )
            });
        });
    }
    group.finish();
}

fn bench_mesh_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("native_accel/mesh_processing");
    for vertices in [1_000_usize, 64_000, 1_000_000] {
        let positions = (0..vertices * 3)
            .map(|index| (index as f32).mul_add(0.001, -1.0))
            .collect::<Vec<_>>();
        let indices = (0..vertices.saturating_sub(2) as u32)
            .step_by(3)
            .flat_map(|index| [index, index + 1, index + 2])
            .collect::<Vec<_>>();
        group.throughput(Throughput::Elements(vertices as u64));
        group.bench_with_input(
            BenchmarkId::new("vertex_normals", vertices),
            &positions,
            |b, positions| {
                let mut normals = vec![0.0; positions.len()];
                b.iter(|| {
                    native_accel::generate_vertex_normals(
                        black_box(positions),
                        black_box(&indices),
                        black_box(&mut normals),
                    )
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("aabb", vertices),
            &positions,
            |b, positions| {
                b.iter(|| black_box(native_accel::calculate_aabb(black_box(positions))));
            },
        );
    }
    group.finish();
}

criterion_group!(
    native_accel_benches,
    bench_transform,
    bench_culling,
    bench_mesh_processing
);
criterion_main!(native_accel_benches);
