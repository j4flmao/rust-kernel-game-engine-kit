//! Minecraft-style voxel world state.

#[derive(Clone, Copy, Debug, Default)]
pub struct Player {
    pub position: [f32; 3],
    pub velocity: [f32; 3],
}

#[derive(Clone, Debug)]
pub struct VoxelWorld {
    pub blocks: usize,
    pub seed: u32,
    pub player: Player,
}

impl VoxelWorld {
    pub fn new() -> Self {
        let blocks = std::env::var("RKE_GPU_STRESS_BLOCKS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(16_384)
            .clamp(1, 250_000);
        Self {
            blocks,
            seed: 0x51f1_aa11,
            player: Player {
                position: [0.0, 12.0, 8.0],
                velocity: [0.0; 3],
            },
        }
    }

    pub fn regenerate(&mut self) {
        self.seed = self
            .seed
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
    }
}
