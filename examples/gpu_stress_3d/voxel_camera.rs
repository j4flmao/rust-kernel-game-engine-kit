use rust_kernel_game_engine_kit::subsystems::renderer_3d::RenderView;

#[derive(Clone, Copy, Debug)]
pub struct OrbitCamera {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            yaw: 0.65,
            pitch: 0.48,
            distance: 14.5,
        }
    }
}

impl OrbitCamera {
    /// Keeps the voxel world at a useful size when the native window is resized or
    /// maximized.  The projection still uses the exact swapchain aspect; this
    /// only adjusts the orbit radius so a very wide client area does not leave
    /// the cube stranded in a small corner.
    pub fn fit_to_viewport(&mut self, width: u32, height: u32) {
        let aspect = width.max(1) as f32 / height.max(1) as f32;
        let vertical_fit = 11.5;
        let narrow_view_fit = vertical_fit / aspect.max(0.55);
        self.distance = vertical_fit.max(narrow_view_fit).min(18.0);
    }

    pub fn view(&self, aspect: f32, width: u32, height: u32) -> RenderView {
        self.view_at(aspect, width, height, [0.0, 0.0, 0.0])
    }

    pub fn view_at(&self, aspect: f32, width: u32, height: u32, target: [f32; 3]) -> RenderView {
        let cos_pitch = self.pitch.cos();
        let eye = [
            target[0] + self.distance * cos_pitch * self.yaw.sin(),
            target[1] + self.distance * self.pitch.sin(),
            target[2] + self.distance * cos_pitch * self.yaw.cos(),
        ];
        let view = look_at(eye, target, [0.0, 1.0, 0.0]);
        let projection = perspective(55.0_f32.to_radians(), aspect.max(0.01), 0.1, 100.0);
        RenderView {
            view_projection: multiply(projection, view),
            viewport_width: width.max(1),
            viewport_height: height.max(1),
            near_plane: 0.1,
            far_plane: 100.0,
        }
    }

    pub fn orbit(&mut self, delta_x: i32, delta_y: i32) {
        self.yaw -= delta_x as f32 * 0.01;
        self.pitch = (self.pitch - delta_y as f32 * 0.01).clamp(-1.4, 1.4);
    }
}

fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> [f32; 16] {
    let f = 1.0 / (fov_y * 0.5).tan();
    [
        f / aspect,
        0.0,
        0.0,
        0.0,
        0.0,
        f,
        0.0,
        0.0,
        0.0,
        0.0,
        far / (near - far),
        -1.0,
        0.0,
        0.0,
        (far * near) / (near - far),
        0.0,
    ]
}

fn look_at(eye: [f32; 3], target: [f32; 3], up: [f32; 3]) -> [f32; 16] {
    let forward = normalize(sub(target, eye));
    let right = normalize(cross(forward, up));
    let corrected_up = cross(right, forward);
    [
        right[0],
        corrected_up[0],
        -forward[0],
        0.0,
        right[1],
        corrected_up[1],
        -forward[1],
        0.0,
        right[2],
        corrected_up[2],
        -forward[2],
        0.0,
        -dot(right, eye),
        -dot(corrected_up, eye),
        dot(forward, eye),
        1.0,
    ]
}

fn multiply(a: [f32; 16], b: [f32; 16]) -> [f32; 16] {
    let mut result = [0.0; 16];
    for column in 0..4 {
        for row in 0..4 {
            result[column * 4 + row] = (0..4)
                .map(|index| a[index * 4 + row] * b[column * 4 + index])
                .sum();
        }
    }
    result
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn scale(value: [f32; 3], factor: f32) -> [f32; 3] {
    [value[0] * factor, value[1] * factor, value[2] * factor]
}

fn subtract(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn normalize(value: [f32; 3]) -> [f32; 3] {
    let length = dot(value, value).sqrt();
    [value[0] / length, value[1] / length, value[2] / length]
}
