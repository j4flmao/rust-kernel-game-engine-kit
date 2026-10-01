//! Bounded, nearest-first chunk residency. CPU work is limited to two chunks per tick.
use super::{
    scene::{self, VoxelFrame, VoxelUpload, LIMIT},
    state::VoxelWorld,
};
use rust_kernel_game_engine_kit::subsystems::renderer::WorldDrawCommand;
use std::collections::BTreeMap;

pub const CHUNKS_PER_FRAME: usize = 2;
#[derive(Default)]
pub struct VoxelScene {
    finite: scene::VoxelScene,
    chunks: BTreeMap<[i32; 2], VoxelFrame>,
    key: Option<(u32, u8)>,
    frame: Option<VoxelFrame>,
    revision: u64,
}
impl VoxelScene {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn build_frame(&mut self, world: &VoxelWorld) -> Result<&VoxelFrame, String> {
        if world.stream_radius == 0 {
            return self.finite.build_frame(world);
        }
        if !world.player.position.iter().all(|v| v.is_finite()) {
            return Err("non-finite streaming position".into());
        }
        let radius = i32::from(world.stream_radius.min(4));
        let center = [world.player.position[0], world.player.position[2]]
            .map(|v| (v.clamp(-65_000.0, 65_000.0).floor() as i32).div_euclid(16));
        let key = (world.seed, radius as u8);
        let mut changed = self.key != Some(key);
        if changed {
            self.chunks.clear();
            self.key = Some(key);
        }
        let before = self.chunks.len();
        self.chunks.retain(|coord, _| {
            (coord[0] - center[0]).abs() <= radius && (coord[1] - center[1]).abs() <= radius
        });
        changed |= before != self.chunks.len();
        let mut missing = Vec::new();
        for x in center[0] - radius..=center[0] + radius {
            for z in center[1] - radius..=center[1] + radius {
                if !self.chunks.contains_key(&[x, z]) {
                    missing.push([x, z]);
                }
            }
        }
        missing.sort_by_key(|c| ((c[0] - center[0]).pow(2) + (c[1] - center[1]).pow(2), *c));
        let built = missing.len().min(CHUNKS_PER_FRAME);
        for coord in missing.iter().take(built) {
            let mesh = scene::build_chunk(world, *coord)?;
            let resident_bytes: usize = self
                .chunks
                .values()
                .map(|c| c.upload.instance_bytes().len())
                .sum();
            if mesh.upload.instance_bytes().len() > LIMIT.saturating_sub(resident_bytes) {
                return Err("streaming residency exceeds 4 MiB budget".into());
            }
            self.chunks.insert(*coord, mesh);
        }
        changed |= built != 0;
        let pending = missing.len() - built;
        if changed || self.frame.is_none() {
            let total: usize = self
                .chunks
                .values()
                .map(|c| c.upload.instance_bytes().len())
                .sum();
            if total > LIMIT {
                return Err("streaming geometry exceeds 4 MiB upload budget".into());
            }
            let mut bytes = Vec::with_capacity(total);
            let mut draws = Vec::new();
            // Compact by direction, preserving the six-draw ABI even across chunks.
            for face in 0..6 {
                let first_instance = (bytes.len() / 80) as u32;
                for chunk in self.chunks.values() {
                    for draw in &chunk.draws {
                        if draw.first_vertex != face * 6 {
                            continue;
                        }
                        let start = draw.first_instance as usize * 80;
                        let end = start + draw.instance_count as usize * 80;
                        bytes.extend_from_slice(&chunk.upload.instance_bytes()[start..end]);
                    }
                }
                let instance_count = (bytes.len() / 80) as u32 - first_instance;
                if instance_count != 0 {
                    draws.push(WorldDrawCommand {
                        vertex_count: 6,
                        first_vertex: face * 6,
                        first_instance,
                        instance_count,
                    });
                }
            }
            self.revision = self.revision.wrapping_add(1);
            self.frame = Some(VoxelFrame {
                upload: VoxelUpload(bytes),
                batch_count: draws.len(),
                draws,
                block_count: self.chunks.values().map(|c| c.block_count).sum(),
                exposed_faces: self.chunks.values().map(|c| c.exposed_faces).sum(),
                revision: self.revision,
                resident_chunks: self.chunks.len(),
                pending_chunks: pending,
                built_this_frame: built,
            });
        } else if let Some(frame) = self.frame.as_mut() {
            frame.built_this_frame = 0;
        }
        Ok(self.frame.as_ref().expect("resident mesh exists"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn world() -> VoxelWorld {
        let mut w = VoxelWorld::new();
        w.stream_radius = 1;
        w.player.position = [0.0, 12.0, 0.0];
        w
    }
    fn settle(scene: &mut VoxelScene, w: &VoxelWorld) {
        for _ in 0..50 {
            let frame = scene.build_frame(w).unwrap();
            assert!(frame.built_this_frame <= CHUNKS_PER_FRAME);
            assert!(frame.resident_chunks <= (usize::from(w.stream_radius) * 2 + 1).pow(2));
            assert!(frame.upload.instance_bytes().len() <= LIMIT);
            if frame.pending_chunks == 0 {
                return;
            }
        }
        panic!("stream failed to settle");
    }
    #[test]
    fn bounded_nearest_first_and_idle_cache() {
        let w = world();
        let mut scene = VoxelScene::new();
        let frame = scene.build_frame(&w).unwrap();
        assert_eq!(frame.resident_chunks, 2);
        assert!(scene.chunks.contains_key(&[0, 0]));
        settle(&mut scene, &w);
        let revision = scene.build_frame(&w).unwrap().revision;
        let frame = scene.build_frame(&w).unwrap();
        assert_eq!(frame.revision, revision);
        assert_eq!(frame.built_this_frame, 0);
    }
    #[test]
    fn negative_boundary_evicts_and_reuses_neighbors() {
        let mut w = world();
        let mut scene = VoxelScene::new();
        settle(&mut scene, &w);
        let pointer = scene.chunks[&[0, 0]].upload.instance_bytes().as_ptr();
        w.player.position[0] = -0.01;
        let frame = scene.build_frame(&w).unwrap();
        assert_eq!(frame.pending_chunks, 1);
        assert!(!scene.chunks.contains_key(&[1, 0]));
        assert_eq!(
            pointer,
            scene.chunks[&[0, 0]].upload.instance_bytes().as_ptr()
        );
        settle(&mut scene, &w);
        assert_eq!(scene.chunks.len(), 9);
    }
    #[test]
    fn teleport_and_regeneration_invalidate() {
        let mut w = world();
        let mut scene = VoxelScene::new();
        settle(&mut scene, &w);
        let revision = scene.frame.as_ref().unwrap().revision;
        w.player.position = [1600.0, 12.0, -1600.0];
        let frame = scene.build_frame(&w).unwrap();
        assert_eq!(frame.resident_chunks, 2);
        assert!(frame.revision > revision);
        settle(&mut scene, &w);
        w.regenerate();
        assert_eq!(scene.build_frame(&w).unwrap().resident_chunks, 2);
    }
    #[test]
    fn maximum_radius_stays_within_budget() {
        let mut w = world();
        w.stream_radius = 4;
        let mut scene = VoxelScene::new();
        settle(&mut scene, &w);
        assert_eq!(scene.chunks.len(), 81);
    }
    #[test]
    fn seam_faces_match_global_terrain_reference() {
        let w = world();
        for coordinate in [[0, 0], [-1, 0], [1, -2]] {
            let mesh = scene::build_chunk(&w, coordinate).unwrap();
            let mut expected = 0;
            for x in coordinate[0] * 16..coordinate[0] * 16 + 16 {
                for z in coordinate[1] * 16..coordinate[1] * 16 + 16 {
                    for y in 0..w.height(x, z) {
                        for d in [
                            [1, 0, 0],
                            [-1, 0, 0],
                            [0, 1, 0],
                            [0, -1, 0],
                            [0, 0, 1],
                            [0, 0, -1],
                        ] {
                            let ny = y + d[1];
                            expected += usize::from(ny < 0 || ny >= w.height(x + d[0], z + d[2]));
                        }
                    }
                }
            }
            assert_eq!(mesh.exposed_faces, expected);
        }
    }
    #[test]
    fn leaving_and_returning_restores_identical_mesh() {
        let mut w = world();
        let mut scene = VoxelScene::new();
        settle(&mut scene, &w);
        let original = scene
            .frame
            .as_ref()
            .unwrap()
            .upload
            .instance_bytes()
            .to_vec();
        w.player.position[0] = 320.0;
        settle(&mut scene, &w);
        w.player.position[0] = 0.0;
        settle(&mut scene, &w);
        assert_eq!(
            scene.frame.as_ref().unwrap().upload.instance_bytes(),
            original
        );
    }
}
