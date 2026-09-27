use criterion::{
    black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput,
};
use rust_kernel_game_engine_kit::kernel::{
    bus::{MessageBus, SpscRing},
    dependency_waves,
    mem::{BumpArena, Pool},
    sync::{HmacSha256Authenticator, SyncKind, SyncPacket},
    World,
};
use rust_kernel_game_engine_kit::subsystems::ui::{
    UiConfig, UiDiagnostics, UiLayoutEngine, UiLength, UiNodeContent, UiNodeKind, UiPaintList,
    UiRect, UiRenderSnapshot, UiTree,
};

fn bench_arena(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory/arena");
    for allocations in [64usize, 256, 1024] {
        group.throughput(Throughput::Elements(allocations as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(allocations),
            &allocations,
            |b, &n| {
                let mut arena = BumpArena::new(n * 64);
                b.iter(|| {
                    arena.reset();
                    for _ in 0..n {
                        black_box(arena.alloc_bytes(24, 8).expect("benchmark arena capacity"));
                    }
                    black_box(arena.used());
                });
            },
        );
    }
    group.finish();
}

fn bench_pool(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory/pool");
    for operations in [64usize, 256, 1024] {
        group.throughput(Throughput::Elements(operations as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(operations),
            &operations,
            |b, &n| {
                let mut pool = Pool::with_capacity(n);
                b.iter(|| {
                    let mut handles = Vec::with_capacity(n);
                    for value in 0..n {
                        handles.push(pool.alloc(black_box(value as u64)).expect("pool capacity"));
                    }
                    for handle in handles {
                        let _: () = pool.free(handle).expect("valid pool handle");
                        black_box(());
                    }
                });
            },
        );
    }
    group.finish();
}

fn bench_ring(c: &mut Criterion) {
    let mut group = c.benchmark_group("transport/spsc-ring");
    for operations in [64usize, 256, 1024] {
        group.throughput(Throughput::Elements(operations as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(operations),
            &operations,
            |b, &n| {
                let ring = SpscRing::with_capacity(n);
                b.iter(|| {
                    for value in 0..n {
                        ring.push(black_box(value as u64)).expect("ring capacity");
                    }
                    for _ in 0..n {
                        black_box(ring.pop().expect("ring item"));
                    }
                });
            },
        );
    }
    group.finish();
}

fn bench_message_bus(c: &mut Criterion) {
    c.bench_function("transport/message-bus/publish-and-pop", |b| {
        let mut bus = MessageBus::new();
        let sender = bus.add_subscriber();
        let receiver = bus.add_subscriber();
        b.iter(|| {
            let envelope = bus.envelope(sender, receiver, 7, Box::new(black_box(42u64)));
            bus.publish(envelope).expect("message bus route");
            black_box(bus.pop_inbox(receiver).expect("published message"));
        });
    });
}

#[allow(dead_code)]
#[derive(Clone, Copy)]
struct Position(f32, f32);

#[allow(dead_code)]
#[derive(Clone, Copy)]
struct Velocity(f32, f32);

fn bench_ecs(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecs");
    for entities in [64usize, 256, 1024] {
        group.throughput(Throughput::Elements(entities as u64));
        group.bench_with_input(
            BenchmarkId::new("spawn-insert-query", entities),
            &entities,
            |b, &n| {
                b.iter_batched(
                    || {
                        let mut world = World::new();
                        for i in 0..n {
                            let entity = world.spawn();
                            world.insert(entity, Position(i as f32, 0.0));
                            world.insert(entity, Velocity(1.0, 1.0));
                        }
                        (world, Vec::with_capacity(n))
                    },
                    |(world, mut scratch)| {
                        world.collect_intersection::<Position, Velocity>(&mut scratch);
                        black_box(scratch.len());
                        black_box(world.entity_count());
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }
    group.finish();
}

fn bench_sync(c: &mut Criterion) {
    let mut group = c.benchmark_group("sync/wire");
    for payload_bytes in [64usize, 1024, 16 * 1024] {
        let packet = SyncPacket {
            sequence: 7,
            ack: 6,
            kind: SyncKind::Delta,
            key: 0xfeed,
            payload: vec![0x5a; payload_bytes],
        };
        let auth = HmacSha256Authenticator::new(b"benchmark-only-key").expect("valid key");
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let nonce = std::array::from_fn(|index| {
            seed.rotate_left((index as u32) & 63) as u8 ^ (index as u8).wrapping_mul(29)
        });
        group.throughput(Throughput::Bytes(payload_bytes as u64));
        group.bench_with_input(
            BenchmarkId::new("encode", payload_bytes),
            &packet,
            |b, packet| {
                b.iter(|| {
                    black_box(
                        rust_kernel_game_engine_kit::kernel::sync::Replicator::encode_packet(
                            black_box(packet),
                        )
                        .unwrap(),
                    );
                });
            },
        );
        group.bench_with_input(BenchmarkId::new("hmac", payload_bytes), &packet, |b, packet| {
            b.iter(|| {
                black_box(
                    rust_kernel_game_engine_kit::kernel::sync::Replicator::encode_authenticated_packet(
                        black_box(packet), nonce, &auth,
                    )
                    .unwrap(),
                );
            });
        });
    }
    group.finish();
}

fn bench_dependency_waves(c: &mut Criterion) {
    let mut group = c.benchmark_group("scheduler/dependency-waves");
    for nodes in [16usize, 64, 256, 1024] {
        let order: Vec<usize> = (0..nodes).collect();
        let deps: Vec<Vec<usize>> = (0..nodes)
            .map(|node| {
                if node == 0 {
                    Vec::new()
                } else {
                    vec![node - 1]
                }
            })
            .collect();
        group.throughput(Throughput::Elements(nodes as u64));
        group.bench_with_input(BenchmarkId::from_parameter(nodes), &nodes, |b, _| {
            b.iter(|| black_box(dependency_waves(black_box(&order), black_box(&deps))));
        });
    }
    group.finish();
}

fn bench_ui_layout_paint(c: &mut Criterion) {
    let mut group = c.benchmark_group("ui/layout-paint");
    for nodes in [32usize, 128, 512] {
        let config = UiConfig {
            max_nodes: nodes.saturating_add(1),
            max_paint_items: nodes.saturating_add(1),
            max_clips: nodes.saturating_add(2),
            max_batches: nodes.saturating_add(1),
            max_upload_bytes: nodes.saturating_mul(128).saturating_add(1024),
            ..UiConfig::default()
        };
        group.throughput(Throughput::Elements(nodes as u64));
        group.bench_with_input(BenchmarkId::from_parameter(nodes), &nodes, |b, &n| {
            b.iter_batched(
                || {
                    let mut tree = UiTree::try_new(config).expect("UI tree capacity");
                    let root = tree.root();
                    for index in 0..n {
                        let mut panel = UiNodeContent::new(UiNodeKind::Panel);
                        panel.style.width = UiLength::Percent(100.0);
                        panel.style.height = UiLength::Points(24.0 + (index % 4) as f32);
                        panel.style.background = [0.1, 0.2, 0.3, 1.0];
                        tree.create(root, panel).expect("UI node capacity");
                    }
                    (
                        tree,
                        UiLayoutEngine::try_new(config).expect("layout capacity"),
                        UiPaintList::try_new(config).expect("paint capacity"),
                        UiDiagnostics::default(),
                    )
                },
                |(mut tree, mut layout, mut paint, mut diagnostics)| {
                    let viewport = UiRect {
                        x: 0.0,
                        y: 0.0,
                        width: 1920.0,
                        height: 1080.0,
                    };
                    layout
                        .layout(&mut tree, viewport, &mut diagnostics)
                        .expect("layout should stay bounded");
                    let plan = paint
                        .build(&tree, viewport, &mut diagnostics)
                        .expect("paint should stay bounded");
                    black_box((plan.item_count, plan.batch_count, plan.upload_bytes));
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn build_ui_benchmark_snapshot(nodes: usize) -> UiRenderSnapshot {
    let config = UiConfig {
        max_nodes: nodes.saturating_add(1),
        max_paint_items: nodes.saturating_add(1),
        max_clips: nodes.saturating_add(2),
        max_batches: nodes.saturating_add(1),
        max_upload_bytes: nodes.saturating_mul(128).saturating_add(1024),
        ..UiConfig::default()
    };
    let mut tree = UiTree::try_new(config).expect("UI tree capacity");
    let root = tree.root();
    for index in 0..nodes {
        let mut panel = UiNodeContent::new(UiNodeKind::Panel);
        panel.style.width = UiLength::Percent(100.0);
        panel.style.height = UiLength::Points(24.0 + (index % 4) as f32);
        panel.style.background = [0.1, 0.2, 0.3, 1.0];
        tree.create(root, panel).expect("UI node capacity");
    }
    let viewport = UiRect {
        x: 0.0,
        y: 0.0,
        width: 1920.0,
        height: 1080.0,
    };
    let mut diagnostics = UiDiagnostics::default();
    let mut layout = UiLayoutEngine::try_new(config).expect("layout capacity");
    layout
        .layout(&mut tree, viewport, &mut diagnostics)
        .expect("layout should stay bounded");
    let mut paint = UiPaintList::try_new(config).expect("paint capacity");
    paint
        .build(&tree, viewport, &mut diagnostics)
        .expect("paint should stay bounded");
    paint.snapshot().expect("snapshot should stay bounded")
}

fn bench_ui_snapshot_encoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("ui/snapshot-encoding");
    for nodes in [32usize, 128, 512] {
        let snapshot = build_ui_benchmark_snapshot(nodes);
        group.throughput(Throughput::Bytes(snapshot.upload.upload_bytes as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(nodes),
            &snapshot,
            |b, snapshot| {
                b.iter(|| black_box(snapshot.encode_bytes().expect("bounded UI encoding")));
            },
        );
    }
    group.finish();
}

fn bench_ui_dirty_upload(c: &mut Criterion) {
    let mut group = c.benchmark_group("ui/dirty-upload");
    for nodes in [32usize, 128, 512] {
        let snapshot = build_ui_benchmark_snapshot(nodes);
        group.throughput(Throughput::Bytes(snapshot.upload.upload_bytes as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(nodes),
            &snapshot,
            |b, snapshot| {
                b.iter(|| {
                    black_box(
                        snapshot
                            .encode_dirty_ranges(Some(snapshot))
                            .expect("bounded dirty upload"),
                    )
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    kernel_benches,
    bench_arena,
    bench_pool,
    bench_ring,
    bench_message_bus,
    bench_ecs,
    bench_sync,
    bench_dependency_waves,
    bench_ui_layout_paint,
    bench_ui_snapshot_encoding,
    bench_ui_dirty_upload,
);
criterion_main!(kernel_benches);
