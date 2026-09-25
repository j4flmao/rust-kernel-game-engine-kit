#![no_main]

use libfuzzer_sys::fuzz_target;
use rust_kernel_game_engine_kit::kernel::ecs::entity::Entity;
use rust_kernel_game_engine_kit::subsystems::renderer::{
    CpuRenderManifest, GpuFramePreparation, IndexedGeometry, RenderInstance, RenderWorld,
};

fuzz_target!(|bytes: &[u8]| {
    // Keep the target bounded: it exercises validation and encoding, not an
    // attacker-controlled allocator benchmark.
    let mut world = RenderWorld::with_capacity(64);
    for (index, chunk) in bytes.chunks_exact(8).take(64).enumerate() {
        let mesh_id = u32::from_le_bytes([chunk[0], chunk[1], 0, 0]);
        let material_id = u32::from_le_bytes([chunk[2], chunk[3], 0, 0]);
        let entity = Entity::new(index as u32, chunk[4] as u32);
        let _ = world.try_push(RenderInstance {
            entity,
            transform: [0.0; 12],
            bounds: [1.0, 1.0, 1.0, 1.0],
            mesh_id,
            material_id,
        });
    }

    let mut manifest = CpuRenderManifest::with_capacity(64, 64);
    if manifest.build(&world).is_err() {
        return;
    }
    let mut preparation = GpuFramePreparation::with_capacity(64, 64);
    if preparation.prepare(&manifest).is_err() {
        return;
    }
    let _ = preparation.command_plan(32);

    let geometry: Vec<IndexedGeometry> = preparation
        .commands()
        .iter()
        .map(|command| IndexedGeometry {
            mesh_id: command.mesh_id,
            index_count: 1 + (command.mesh_id & 63),
            first_index: 0,
            vertex_offset: 0,
        })
        .collect();
    let mut indirect = vec![0_u8; geometry.len() * 20];
    let _ = preparation.encode_indexed_commands(&geometry, &mut indirect);
    let mut instances = vec![0_u8; preparation.instances().len() * 80];
    let _ = preparation.encode_instance_records(&mut instances);
});
