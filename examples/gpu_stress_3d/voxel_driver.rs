//! Input driver for the voxel world and player movement.

use super::state::VoxelWorld;

pub struct VoxelDriver {
    pub world: VoxelWorld,
    pub forward: f32,
    pub strafe: f32,
}

impl VoxelDriver {
    pub fn new() -> Self {
        Self {
            world: VoxelWorld::new(),
            forward: 0.0,
            strafe: 0.0,
        }
    }

    pub fn tick(&mut self, seconds: f32) {
        let speed = 8.0;
        self.world.player.position[0] += self.strafe * speed * seconds;
        self.world.player.position[2] += self.forward * speed * seconds;
        self.world.player.velocity[1] -= 18.0 * seconds;
        self.world.player.position[1] += self.world.player.velocity[1] * seconds;
        if self.world.player.position[1] < 4.0 {
            self.world.player.position[1] = 4.0;
            self.world.player.velocity[1] = 0.0;
        }
    }

    pub fn jump(&mut self) {
        if self.world.player.position[1] <= 4.01 {
            self.world.player.velocity[1] = 7.0;
        }
    }
}
