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
    /// Zero preserves the finite benchmark; 1..=4 enables bounded streaming.
    pub stream_radius: u8,
    /// Mesher-only origin for a 16x16 terrain column chunk.
    pub region: Option<[i32; 2]>,
}

impl VoxelWorld {
    /// Approximate block budget; terrain columns remain complete.
    pub fn side(&self) -> i32 {
        if self.region.is_some() {
            return 16;
        }
        ((self.blocks as f64 / 8.0).sqrt().ceil() as i32).max(1)
    }
    pub fn height(&self, x: i32, z: i32) -> i32 {
        let [ox, oz] = self.region.unwrap_or([0, 0]);
        let (x, z) = (x + ox, z + oz);
        5 + (((x.div_euclid(4) as u32).wrapping_mul(17)
            ^ (z.div_euclid(4) as u32).wrapping_mul(31)
            ^ self.seed)
            & 7) as i32
    }
    pub fn material(&self, p: [i32; 3]) -> u8 {
        if p[1] < 0
            || (self.region.is_none()
                && (p[0] < 0 || p[2] < 0 || p[0] >= self.side() || p[2] >= self.side()))
        {
            return 0;
        }
        let height = self.height(p[0], p[2]);
        if p[1] >= height {
            0
        } else if p[1] == height - 1 {
            1
        } else if p[1] >= height - 3 {
            2
        } else {
            3
        }
    }
    pub fn floor_at(&self, x: f32, z: f32) -> f32 {
        let half = if self.stream_radius == 0 {
            self.side() as f32 * 0.5
        } else {
            0.0
        };
        self.height((x + half).floor() as i32, (z + half).floor() as i32) as f32 + 1.7
    }
    pub fn new() -> Self {
        let blocks = std::env::var("RKE_GPU_STRESS_BLOCKS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(16_384)
            .clamp(1, 250_000);
        Self {
            blocks,
            stream_radius: std::env::var("RKE_GPU_STRESS_STREAM_RADIUS")
                .ok()
                .and_then(|v| v.parse::<u8>().ok())
                .unwrap_or(0)
                .min(4),
            region: None,
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
