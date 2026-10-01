//! Cached chunk-local greedy surface meshes, independent of Rubik.
use super::state::VoxelWorld;
use rust_kernel_game_engine_kit::subsystems::renderer::WorldDrawCommand;
pub const LIMIT: usize = 4 * 1024 * 1024;
#[derive(Default)]
pub struct VoxelScene {
    key: Option<(usize, u32, Option<[i32; 2]>)>,
    revision: u64,
    frame: Option<VoxelFrame>,
}
pub struct VoxelUpload(pub Vec<u8>);
impl VoxelUpload {
    pub fn instance_bytes(&self) -> &[u8] {
        &self.0
    }
}
pub struct VoxelFrame {
    pub upload: VoxelUpload,
    pub draws: Vec<WorldDrawCommand>,
    pub block_count: u32,
    pub batch_count: usize,
    pub exposed_faces: usize,
    pub revision: u64,
    pub resident_chunks: usize,
    pub pending_chunks: usize,
    pub built_this_frame: usize,
}
impl VoxelScene {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn build_frame(&mut self, state: &VoxelWorld) -> Result<&VoxelFrame, String> {
        let key = (state.blocks, state.seed, state.region);
        if self.key != Some(key) {
            let side = state.side();
            let mut groups: [Vec<u8>; 6] = Default::default();
            let mut exposed_faces = 0;
            let mut block_count = 0;
            for x in 0..side {
                for z in 0..side {
                    block_count += state.height(x, z) as u32;
                }
            }
            for cx in (0..side).step_by(16) {
                for cz in (0..side).step_by(16) {
                    for (face, group) in groups.iter_mut().enumerate() {
                        let (axis, u, v, sign) = match face {
                            0 => (2, 0, 1, 1),
                            1 => (2, 0, 1, -1),
                            2 => (0, 2, 1, -1),
                            3 => (0, 2, 1, 1),
                            4 => (1, 0, 2, 1),
                            _ => (1, 0, 2, -1),
                        };
                        let origin = [cx, 0, cz];
                        for slice in 0..16 {
                            let mut mask = [0_u8; 256];
                            for j in 0..16 {
                                for i in 0..16 {
                                    let mut p = origin;
                                    p[axis] += slice;
                                    p[u] += i;
                                    p[v] += j;
                                    let material = state.material(p);
                                    let mut neighbor = p;
                                    neighbor[axis] += sign;
                                    if material != 0 && state.material(neighbor) == 0 {
                                        mask[(j * 16 + i) as usize] = material;
                                        exposed_faces += 1;
                                    }
                                }
                            }
                            for (i, j, w, h, material) in rectangles(&mut mask) {
                                let mut p = origin.map(|n| n as f32);
                                p[axis] += slice as f32 + if sign > 0 { 1.0 } else { 0.0 };
                                p[u] += i as f32 + w as f32 * 0.5;
                                p[v] += j as f32 + h as f32 * 0.5;
                                if let Some([ox, oz]) = state.region {
                                    p[0] += ox as f32;
                                    p[2] += oz as f32;
                                } else {
                                    p[0] -= side as f32 * 0.5;
                                    p[2] -= side as f32 * 0.5;
                                }
                                let mut s = [0.0; 3];
                                s[u] = w as f32 * 0.5;
                                s[v] = h as f32 * 0.5;
                                encode(group, p, s, material);
                                if group.len() > LIMIT {
                                    return Err("voxel mesh exceeds bounded upload budget".into());
                                }
                            }
                        }
                    }
                }
            }
            let total: usize = groups.iter().map(Vec::len).sum();
            if total > LIMIT {
                return Err("voxel mesh exceeds 4 MiB upload budget".into());
            }
            let mut bytes = Vec::with_capacity(total);
            let mut draws = Vec::new();
            for (face, group) in groups.iter().enumerate() {
                if group.is_empty() {
                    continue;
                }
                draws.push(WorldDrawCommand {
                    vertex_count: 6,
                    instance_count: (group.len() / 80) as u32,
                    first_vertex: face as u32 * 6,
                    first_instance: (bytes.len() / 80) as u32,
                });
                bytes.extend_from_slice(group);
            }
            self.revision = self.revision.wrapping_add(1);
            self.frame = Some(VoxelFrame {
                upload: VoxelUpload(bytes),
                batch_count: draws.len(),
                draws,
                block_count,
                exposed_faces,
                revision: self.revision,
                resident_chunks: ((side + 15) / 16).pow(2) as usize,
                pending_chunks: 0,
                built_this_frame: ((side + 15) / 16).pow(2) as usize,
            });
            self.key = Some(key);
        } else if let Some(frame) = self.frame.as_mut() {
            frame.built_this_frame = 0;
        }
        Ok(self.frame.as_ref().expect("mesh built for cache key"))
    }
}
pub fn build_chunk(state: &VoxelWorld, coordinate: [i32; 2]) -> Result<VoxelFrame, String> {
    let mut local = state.clone();
    local.region = Some([coordinate[0] * 16, coordinate[1] * 16]);
    let mut scene = VoxelScene::new();
    scene.build_frame(&local)?;
    Ok(scene.frame.expect("chunk mesh built"))
}
fn encode(bytes: &mut Vec<u8>, p: [f32; 3], s: [f32; 3], material: u8) {
    for value in [
        s[0],
        0.0,
        0.0,
        p[0],
        0.0,
        s[1],
        0.0,
        p[1],
        0.0,
        0.0,
        s[2],
        p[2],
        p[0],
        p[1],
        p[2],
        s[0].hypot(s[1]).hypot(s[2]),
    ] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in [0_u32, 0, 0, 0x8000_0000 | u32::from(material - 1)] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
}
fn rectangles(mask: &mut [u8; 256]) -> Vec<(usize, usize, usize, usize, u8)> {
    let mut result = Vec::new();
    for j in 0..16 {
        for i in 0..16 {
            let material = mask[j * 16 + i];
            if material == 0 {
                continue;
            }
            let mut w = 1;
            while i + w < 16 && mask[j * 16 + i + w] == material {
                w += 1;
            }
            let mut h = 1;
            while j + h < 16 && (0..w).all(|x| mask[(j + h) * 16 + i + x] == material) {
                h += 1;
            }
            for y in 0..h {
                for x in 0..w {
                    mask[(j + y) * 16 + i + x] = 0;
                }
            }
            result.push((i, j, w, h, material));
        }
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn greedy_area_matches_all_exposed_faces() {
        for blocks in [1, 2048, 100_000, 250_000] {
            let mut state = VoxelWorld::new();
            state.blocks = blocks;
            let mut scene = VoxelScene::new();
            let frame = scene.build_frame(&state).unwrap();
            let mut area = 0.0_f32;
            for draw in &frame.draws {
                let face = draw.first_vertex / 6;
                let axes = match face {
                    0 | 1 => (0, 5),
                    2 | 3 => (5, 10),
                    _ => (0, 10),
                };
                for index in draw.first_instance..draw.first_instance + draw.instance_count {
                    let record = &frame.upload.0[index as usize * 80..][..80];
                    let scalar = |word: usize| {
                        f32::from_le_bytes(record[word * 4..word * 4 + 4].try_into().unwrap())
                    };
                    area += 4.0 * scalar(axes.0) * scalar(axes.1);
                }
            }
            assert_eq!(area as usize, frame.exposed_faces);
            let side = state.side();
            let mut expected = 0;
            for x in 0..side {
                for z in 0..side {
                    for y in 0..state.height(x, z) {
                        for d in [
                            [1, 0, 0],
                            [-1, 0, 0],
                            [0, 1, 0],
                            [0, -1, 0],
                            [0, 0, 1],
                            [0, 0, -1],
                        ] {
                            expected +=
                                usize::from(state.material([x + d[0], y + d[1], z + d[2]]) == 0);
                        }
                    }
                }
            }
            assert_eq!(frame.exposed_faces, expected);
        }
    }
    #[test]
    fn solid_plane_merges() {
        assert_eq!(rectangles(&mut [1; 256]), vec![(0, 0, 16, 16, 1)]);
    }
    #[test]
    fn checkerboard_preserves_materials() {
        let mut mask = std::array::from_fn(|i| 1 + ((i / 16 + i % 16) % 2) as u8);
        assert_eq!(rectangles(&mut mask).len(), 256);
        assert!(mask.iter().all(|&v| v == 0));
    }
    #[test]
    fn cached_mesh_is_bounded() {
        let mut state = VoxelWorld::new();
        state.blocks = 16384;
        let mut scene = VoxelScene::new();
        let frame = scene.build_frame(&state).unwrap();
        assert_eq!(frame.draws.len(), 6);
        assert!(frame.upload.0.len() / 80 < frame.exposed_faces);
        assert!(frame.upload.0.len() <= LIMIT);
        let ptr = frame.upload.0.as_ptr();
        assert_eq!(ptr, scene.build_frame(&state).unwrap().upload.0.as_ptr());
        state.regenerate();
        assert!(scene.build_frame(&state).is_ok());
    }
}
