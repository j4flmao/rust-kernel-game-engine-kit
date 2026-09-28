use super::driver::ActiveRotation;
use super::state::{face_normals, Axis, CubeState, Cubie, RotationCommand};
use rust_kernel_game_engine_kit::kernel::ecs::World;
use rust_kernel_game_engine_kit::subsystems::renderer::{
    CpuRenderManifest, GpuFramePreparation, GpuFrameUpload, IndexedGeometry, RenderMaterial,
    RenderTransform, WorldDrawCommand,
};
use rust_kernel_game_engine_kit::subsystems::renderer_3d::{
    MaterialRecord, MaterialTable, MeshTable,
};

// Leave a small dark seam between adjacent cubies so the three visible sides
// read as a real 3x3 sticker grid instead of a single solid block.
const CUBIE_GAP: f32 = 2.02;
const CUBIE_SCALE: f32 = 0.98;

pub struct RubikScene {
    pub(crate) state: CubeState,
}

pub struct RubikFrame {
    pub upload: GpuFrameUpload,
    pub draws: Vec<WorldDrawCommand>,
    pub instance_count: u32,
    pub batch_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PickHit {
    pub coordinate: [i32; 3],
    /// Outward world-space normal of the face under the cursor.
    pub normal: [i32; 3],
}

impl RubikScene {
    pub fn new() -> Self {
        Self {
            state: CubeState::new(),
        }
    }

    #[allow(dead_code)]
    pub fn turn_layer(&mut self, axis: usize, layer: i32, direction: i32) -> bool {
        let axis = match axis {
            0 => Axis::X,
            1 => Axis::Y,
            2 => Axis::Z,
            _ => return false,
        };
        let Ok(layer) = i8::try_from(layer) else {
            return false;
        };
        let Ok(direction) = i8::try_from(direction) else {
            return false;
        };
        let Some(command) = RotationCommand::new(axis, layer, direction) else {
            return false;
        };
        self.state.apply_move(command)
    }

    #[allow(dead_code)]
    pub fn state(&self) -> &CubeState {
        &self.state
    }

    pub fn sync_state(&mut self, state: &CubeState) {
        self.state = state.clone();
    }

    pub fn pick_face(
        &self,
        cursor_x: i32,
        cursor_y: i32,
        view_projection: [f32; 16],
        viewport_width: u32,
        viewport_height: u32,
    ) -> Option<PickHit> {
        let mut best: Option<(PickHit, f32, f32)> = None;
        for cubie in self.state.cubies() {
            let position = [
                cubie.coordinate[0] as f32 * CUBIE_GAP,
                cubie.coordinate[1] as f32 * CUBIE_GAP,
                cubie.coordinate[2] as f32 * CUBIE_GAP,
            ];
            for local_normal in face_normals() {
                let normal = multiply_vec(cubie.orientation, local_normal);
                let face_center = [
                    position[0] + normal[0] as f32 * CUBIE_SCALE,
                    position[1] + normal[1] as f32 * CUBIE_SCALE,
                    position[2] + normal[2] as f32 * CUBIE_SCALE,
                ];
                let Some((screen_x, screen_y, depth)) = project_point(
                    face_center,
                    view_projection,
                    viewport_width,
                    viewport_height,
                ) else {
                    continue;
                };
                let dx = screen_x - cursor_x as f32;
                let dy = screen_y - cursor_y as f32;
                let distance_squared = dx * dx + dy * dy;
                if distance_squared > 56.0 * 56.0 {
                    continue;
                }
                let hit = PickHit {
                    coordinate: cubie.coordinate.map(i32::from),
                    normal: normal.map(i32::from),
                };
                let is_better = best
                    .map(|(_, best_distance, best_depth)| {
                        depth < best_depth - 0.0001
                            || ((depth - best_depth).abs() <= 0.0001
                                && distance_squared < best_distance)
                    })
                    .unwrap_or(true);
                if is_better {
                    best = Some((hit, distance_squared, depth));
                }
            }
        }
        best.map(|(hit, _, _)| hit)
    }

    #[allow(dead_code)]
    pub fn build_frame(&self) -> Result<RubikFrame, String> {
        self.build_frame_for(&self.state, None)
    }

    pub fn build_frame_for(
        &self,
        state: &CubeState,
        active: Option<ActiveRotation>,
    ) -> Result<RubikFrame, String> {
        let mut meshes = MeshTable::try_with_capacity(4, 16 * 1024, 16 * 1024)
            .map_err(|error| error.to_string())?;
        // The direct bring-up shader expands the cube from gl_VertexIndex. The
        // mesh resource is still validated through the kernel table contract.
        let cube_vertices = [0_u8; 36 * 12];
        let cube_indices = [0_u8; 36 * core::mem::size_of::<u32>()];
        let cube = meshes
            .try_insert(&cube_vertices, 12, &cube_indices, [0.0, 0.0, 0.0, 1.0])
            .map_err(|error| error.to_string())?;
        let cube_record = meshes.get(cube).map_err(|error| error.to_string())?;

        let mut materials =
            MaterialTable::try_with_capacity(6).map_err(|error| error.to_string())?;
        for color in [
            [0.92, 0.12, 0.12, 1.0],
            [0.98, 0.90, 0.16, 1.0],
            [0.95, 0.95, 0.95, 1.0],
            [0.12, 0.32, 0.92, 1.0],
            [0.95, 0.48, 0.08, 1.0],
            [0.10, 0.68, 0.28, 1.0],
        ] {
            materials
                .try_insert(MaterialRecord {
                    base_color: color,
                    metallic: 0.0,
                    roughness: 0.55,
                })
                .map_err(|error| error.to_string())?;
        }

        let mut world = World::new();
        for cubie in state.cubies() {
            let entity = world
                .try_spawn()
                .map_err(|error| format!("spawn cubie: {error:?}"))?;
            let position = [
                cubie.coordinate[0] as f32 * CUBIE_GAP,
                cubie.coordinate[1] as f32 * CUBIE_GAP,
                cubie.coordinate[2] as f32 * CUBIE_GAP,
            ];
            world
                .try_insert(
                    entity,
                    RenderTransform {
                        matrix: model_matrix(cubie, active),
                        bounds: [position[0], position[1], position[2], 1.7],
                    },
                )
                .map_err(|error| format!("insert transform: {error:?}"))?;
            world
                .try_insert(
                    entity,
                    RenderMaterial {
                        mesh_id: cube.index(),
                        material_id: pack_face_colors(cubie.face_colors),
                    },
                )
                .map_err(|error| format!("insert material: {error:?}"))?;
        }

        let mut render_world =
            rust_kernel_game_engine_kit::subsystems::renderer::RenderWorld::try_with_capacity(64)
                .map_err(|error| format!("render world: {error:?}"))?;
        let extracted = render_world
            .extract_typed(&world)
            .map_err(|error| format!("extract render world: {error:?}"))?;
        if extracted.extracted != 27 {
            return Err(format!(
                "expected 27 render instances, extracted {}",
                extracted.extracted
            ));
        }

        let mut manifest = CpuRenderManifest::try_with_capacity(64, 32)
            .map_err(|error| format!("manifest: {error:?}"))?;
        manifest
            .build(&render_world)
            .map_err(|error| format!("manifest build: {error:?}"))?;
        let mut preparation = GpuFramePreparation::try_with_capacity(64, 32)
            .map_err(|error| format!("GPU preparation: {error:?}"))?;
        preparation
            .prepare(&manifest)
            .map_err(|error| format!("GPU preparation: {error:?}"))?;
        let upload = GpuFrameUpload::try_build(
            &preparation,
            &[IndexedGeometry {
                mesh_id: cube.index(),
                index_count: cube_record.index_count,
                first_index: 0,
                vertex_offset: 0,
            }],
            64,
        )
        .map_err(|error| format!("GPU upload: {error:?}"))?;
        let plan = upload.plan();
        if plan.instance_count != 27 || plan.draw_count == 0 || plan.draw_count > 27 {
            return Err(format!(
                "unexpected GPU plan: {} instances, {} draws",
                plan.instance_count, plan.draw_count
            ));
        }

        let mut draws = Vec::new();
        draws
            .try_reserve(preparation.commands().len())
            .map_err(|_| "world draw command allocation failed".to_owned())?;
        for command in preparation.commands() {
            draws.push(WorldDrawCommand {
                vertex_count: cube_record.index_count,
                instance_count: command.instance_count,
                first_vertex: 0,
                first_instance: command.first_instance,
            });
        }

        Ok(RubikFrame {
            upload,
            draws,
            instance_count: plan.instance_count,
            batch_count: manifest.batches().len(),
        })
    }
}

fn pack_face_colors(colors: [u8; 6]) -> u32 {
    colors
        .iter()
        .enumerate()
        .fold(0_u32, |packed, (face, color)| {
            packed | (u32::from(*color & 7) << (face * 3))
        })
}

fn model_matrix(cubie: &Cubie, active: Option<ActiveRotation>) -> [f32; 12] {
    let mut position = [
        cubie.coordinate[0] as f32 * CUBIE_GAP,
        cubie.coordinate[1] as f32 * CUBIE_GAP,
        cubie.coordinate[2] as f32 * CUBIE_GAP,
    ];
    let mut orientation = [
        [
            cubie.orientation[0][0] as f32,
            cubie.orientation[0][1] as f32,
            cubie.orientation[0][2] as f32,
        ],
        [
            cubie.orientation[1][0] as f32,
            cubie.orientation[1][1] as f32,
            cubie.orientation[1][2] as f32,
        ],
        [
            cubie.orientation[2][0] as f32,
            cubie.orientation[2][1] as f32,
            cubie.orientation[2][2] as f32,
        ],
    ];
    if let Some(active) = active {
        let axis = active.command.axis.index();
        if cubie.coordinate[axis] == active.command.layer {
            let progress = active.progress.clamp(0.0, 1.0);
            let eased = 1.0 - (1.0 - progress).powi(3);
            let angle = eased * active.command.direction as f32 * core::f32::consts::FRAC_PI_2;
            let rotation = float_rotation(active.command.axis, angle);
            position = multiply_float_vec(rotation, position);
            orientation = multiply_float(rotation, orientation);
        }
    }
    [
        orientation[0][0] * CUBIE_SCALE,
        orientation[0][1] * CUBIE_SCALE,
        orientation[0][2] * CUBIE_SCALE,
        position[0],
        orientation[1][0] * CUBIE_SCALE,
        orientation[1][1] * CUBIE_SCALE,
        orientation[1][2] * CUBIE_SCALE,
        position[1],
        orientation[2][0] * CUBIE_SCALE,
        orientation[2][1] * CUBIE_SCALE,
        orientation[2][2] * CUBIE_SCALE,
        position[2],
    ]
}

fn float_rotation(axis: Axis, angle: f32) -> [[f32; 3]; 3] {
    let (sin, cos) = angle.sin_cos();
    match axis {
        Axis::X => [[1.0, 0.0, 0.0], [0.0, cos, -sin], [0.0, sin, cos]],
        Axis::Y => [[cos, 0.0, sin], [0.0, 1.0, 0.0], [-sin, 0.0, cos]],
        Axis::Z => [[cos, -sin, 0.0], [sin, cos, 0.0], [0.0, 0.0, 1.0]],
    }
}

fn multiply_float(a: [[f32; 3]; 3], b: [[f32; 3]; 3]) -> [[f32; 3]; 3] {
    let mut result = [[0.0; 3]; 3];
    for row in 0..3 {
        for column in 0..3 {
            result[row][column] = (0..3).map(|index| a[row][index] * b[index][column]).sum();
        }
    }
    result
}

fn multiply_float_vec(matrix: [[f32; 3]; 3], vector: [f32; 3]) -> [f32; 3] {
    [
        matrix[0][0] * vector[0] + matrix[0][1] * vector[1] + matrix[0][2] * vector[2],
        matrix[1][0] * vector[0] + matrix[1][1] * vector[1] + matrix[1][2] * vector[2],
        matrix[2][0] * vector[0] + matrix[2][1] * vector[1] + matrix[2][2] * vector[2],
    ]
}

fn multiply_vec(matrix: [[i8; 3]; 3], vector: [i8; 3]) -> [i8; 3] {
    [
        matrix[0][0] * vector[0] + matrix[0][1] * vector[1] + matrix[0][2] * vector[2],
        matrix[1][0] * vector[0] + matrix[1][1] * vector[1] + matrix[1][2] * vector[2],
        matrix[2][0] * vector[0] + matrix[2][1] * vector[1] + matrix[2][2] * vector[2],
    ]
}

fn project_point(
    point: [f32; 3],
    matrix: [f32; 16],
    viewport_width: u32,
    viewport_height: u32,
) -> Option<(f32, f32, f32)> {
    let clip_x = matrix[0] * point[0] + matrix[4] * point[1] + matrix[8] * point[2] + matrix[12];
    let clip_y = matrix[1] * point[0] + matrix[5] * point[1] + matrix[9] * point[2] + matrix[13];
    let clip_z = matrix[2] * point[0] + matrix[6] * point[1] + matrix[10] * point[2] + matrix[14];
    let clip_w = matrix[3] * point[0] + matrix[7] * point[1] + matrix[11] * point[2] + matrix[15];
    if !clip_w.is_finite() || clip_w <= 0.0001 {
        return None;
    }
    let ndc_x = clip_x / clip_w;
    let ndc_y = clip_y / clip_w;
    let ndc_z = clip_z / clip_w;
    if !ndc_x.is_finite() || !ndc_y.is_finite() || !ndc_z.is_finite() {
        return None;
    }
    Some((
        (ndc_x * 0.5 + 0.5) * viewport_width.max(1) as f32,
        (1.0 - (ndc_y * 0.5 + 0.5)) * viewport_height.max(1) as f32,
        ndc_z,
    ))
}

#[cfg(test)]
mod tests {
    use super::RubikScene;

    #[test]
    fn four_quarter_turns_restore_a_layer() {
        for axis in 0..3 {
            for layer in -1..=1 {
                let mut scene = RubikScene::new();
                let initial = scene.state.clone();
                for _ in 0..4 {
                    assert!(scene.turn_layer(axis, layer, 1));
                }
                assert_eq!(scene.state, initial);
            }
        }
    }

    #[test]
    fn a_turn_preserves_the_cubie_count() {
        let mut scene = RubikScene::new();
        assert!(scene.turn_layer(2, 1, -1));
        assert_eq!(scene.state.cubies().len(), 27);
    }

    #[test]
    fn layer_turn_moves_corner_stickers_between_world_faces() {
        let mut scene = RubikScene::new();
        assert!(scene.turn_layer(0, 1, 1));

        let cubie = scene
            .state
            .cubies()
            .iter()
            .find(|cubie| cubie.coordinate == [1, -1, 1])
            .expect("turned front-right-upper corner must move to front-right-lower");
        // The local sticker slots stay attached to the cubie while the
        // orientation matrix carries them to their new world faces.
        assert_eq!(cubie.face_colors[0], 2);
        assert_eq!(cubie.face_colors[3], 3);
        assert_eq!(cubie.face_colors[4], 4);
    }
}
