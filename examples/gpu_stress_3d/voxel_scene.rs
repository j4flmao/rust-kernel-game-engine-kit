//! Minecraft-style voxel terrain extraction.

use super::state::VoxelWorld;
use rust_kernel_game_engine_kit::kernel::ecs::World;
use rust_kernel_game_engine_kit::subsystems::renderer::{
    CpuRenderManifest, GpuFramePreparation, GpuFrameUpload, IndexedGeometry, RenderMaterial,
    RenderTransform, WorldDrawCommand,
};
use rust_kernel_game_engine_kit::subsystems::renderer_3d::{
    MaterialRecord, MaterialTable, MeshTable,
};

const BLOCK_GAP: f32 = 1.02;

pub struct VoxelScene;

pub struct VoxelFrame {
    pub upload: GpuFrameUpload,
    pub draws: Vec<WorldDrawCommand>,
    pub block_count: u32,
    pub batch_count: usize,
}

impl VoxelScene {
    pub fn new() -> Self {
        Self
    }

    pub fn build_frame(&self, state: &VoxelWorld) -> Result<VoxelFrame, String> {
        let capacity = state.blocks;
        let mut meshes =
            MeshTable::try_with_capacity(1, 16 * 1024, 16 * 1024).map_err(|e| e.to_string())?;
        let mesh = meshes
            .try_insert(&[0_u8; 432], 12, &[0_u8; 144], [0.0, 0.0, 0.0, 1.0])
            .map_err(|e| e.to_string())?;
        let mesh_record = meshes.get(mesh).map_err(|e| e.to_string())?;
        let mut materials = MaterialTable::try_with_capacity(6).map_err(|e| e.to_string())?;
        for color in [
            [0.18, 0.62, 0.12, 1.0],
            [0.35, 0.20, 0.08, 1.0],
            [0.45, 0.45, 0.45, 1.0],
            [0.12, 0.32, 0.08, 1.0],
            [0.10, 0.42, 0.72, 1.0],
            [0.72, 0.72, 0.72, 1.0],
        ] {
            materials
                .try_insert(MaterialRecord {
                    base_color: color,
                    metallic: 0.0,
                    roughness: 0.9,
                })
                .map_err(|e| e.to_string())?;
        }
        let width = 256_usize;
        let depth = capacity.div_ceil(width).max(1);
        let mut ecs = World::new();
        for block in 0..capacity {
            let x = block % width;
            let z = block / width;
            let height = 2 + ((x * 17 + z * 31 + state.seed as usize) % 18);
            let y = block / width % height;
            let px = (x as f32 - width as f32 * 0.5) * BLOCK_GAP;
            let py = y as f32 * BLOCK_GAP - 2.0;
            let pz = (z as f32 - depth as f32 * 0.5) * BLOCK_GAP;
            let entity = ecs.try_spawn().map_err(|e| format!("spawn voxel: {e:?}"))?;
            let matrix = [0.98, 0.0, 0.0, px, 0.0, 0.98, 0.0, py, 0.0, 0.0, 0.98, pz];
            ecs.try_insert(
                entity,
                RenderTransform {
                    matrix,
                    bounds: [px, py, pz, 0.9],
                },
            )
            .map_err(|e| format!("insert transform: {e:?}"))?;
            let material = if y + 1 == height {
                0
            } else if y < 2 {
                1
            } else {
                2
            };
            ecs.try_insert(
                entity,
                RenderMaterial {
                    mesh_id: mesh.index(),
                    material_id: material,
                },
            )
            .map_err(|e| format!("insert material: {e:?}"))?;
        }
        let mut render_world =
            rust_kernel_game_engine_kit::subsystems::renderer::RenderWorld::try_with_capacity(
                capacity,
            )
            .map_err(|e| format!("render world: {e:?}"))?;
        let extracted = render_world
            .extract_typed(&ecs)
            .map_err(|e| format!("extract voxel world: {e:?}"))?;
        if extracted.extracted != capacity {
            return Err(format!(
                "expected {capacity} voxels, got {}",
                extracted.extracted
            ));
        }
        let mut manifest = CpuRenderManifest::try_with_capacity(capacity, 64)
            .map_err(|e| format!("manifest: {e:?}"))?;
        manifest
            .build(&render_world)
            .map_err(|e| format!("manifest: {e:?}"))?;
        let mut preparation = GpuFramePreparation::try_with_capacity(capacity, 64)
            .map_err(|e| format!("preparation: {e:?}"))?;
        preparation
            .prepare(&manifest)
            .map_err(|e| format!("preparation: {e:?}"))?;
        let upload = GpuFrameUpload::try_build(
            &preparation,
            &[IndexedGeometry {
                mesh_id: mesh.index(),
                index_count: mesh_record.index_count,
                first_index: 0,
                vertex_offset: 0,
            }],
            u32::try_from(capacity).map_err(|_| "voxel count overflow".to_owned())?,
        )
        .map_err(|e| format!("upload: {e:?}"))?;
        let mut draws = Vec::new();
        draws
            .try_reserve(preparation.commands().len())
            .map_err(|_| "draw allocation failed")?;
        for command in preparation.commands() {
            draws.push(WorldDrawCommand {
                vertex_count: mesh_record.index_count,
                instance_count: command.instance_count,
                first_vertex: 0,
                first_instance: command.first_instance,
            });
        }
        let block_count = upload.plan().instance_count;
        Ok(VoxelFrame {
            upload,
            draws,
            block_count,
            batch_count: manifest.batches().len(),
        })
    }
}
