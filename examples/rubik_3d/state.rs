//! Canonical Rubik state.
//!
//! The render scene is a projection of this module.  A move never edits GPU
//! material data directly: it permutes cubie positions/orientations, then
//! rebuilds the bounded 54-facelet view used by validation and future solver
//! code.

pub const CUBIE_COUNT: usize = 27;
pub const FACELET_COUNT: usize = 54;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    pub const fn index(self) -> usize {
        match self {
            Self::X => 0,
            Self::Y => 1,
            Self::Z => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RotationCommand {
    pub axis: Axis,
    pub layer: i8,
    pub direction: i8,
}

impl RotationCommand {
    pub const fn new(axis: Axis, layer: i8, direction: i8) -> Option<Self> {
        if layer < -1 || layer > 1 || direction != -1 && direction != 1 {
            return None;
        }
        Some(Self {
            axis,
            layer,
            direction,
        })
    }

    pub const fn inverse(self) -> Self {
        Self {
            direction: -self.direction,
            ..self
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cubie {
    pub coordinate: [i8; 3],
    pub orientation: [[i8; 3]; 3],
    /// Local face order: +Z, -Z, -X, +X, +Y, -Y.
    pub face_colors: [u8; 6],
}

#[derive(Clone, Debug)]
pub struct CubeState {
    cubies: [Cubie; CUBIE_COUNT],
    facelets: [u8; FACELET_COUNT],
    history: Vec<RotationCommand>,
    redo: Vec<RotationCommand>,
}

impl PartialEq for CubeState {
    fn eq(&self, other: &Self) -> bool {
        self.cubies == other.cubies && self.facelets == other.facelets
    }
}

impl Eq for CubeState {}

impl CubeState {
    pub fn new() -> Self {
        let cubies = core::array::from_fn(|index| {
            let x = (index % 3) as i8 - 1;
            let y = ((index / 3) % 3) as i8 - 1;
            let z = (index / 9) as i8 - 1;
            let coordinate = [x, y, z];
            Cubie {
                coordinate,
                orientation: identity_orientation(),
                face_colors: initial_face_colors(coordinate),
            }
        });
        let mut state = Self {
            cubies,
            facelets: [6; FACELET_COUNT],
            history: Vec::with_capacity(512),
            redo: Vec::with_capacity(512),
        };
        state.sync_facelets();
        state
    }

    pub fn cubies(&self) -> &[Cubie; CUBIE_COUNT] {
        &self.cubies
    }

    #[allow(dead_code)]
    pub fn facelets(&self) -> &[u8; FACELET_COUNT] {
        &self.facelets
    }

    #[allow(dead_code)]
    pub fn history_len(&self) -> usize {
        self.history.len()
    }

    pub fn history(&self) -> &[RotationCommand] {
        &self.history
    }

    #[allow(dead_code)]
    pub fn is_solved(&self) -> bool {
        (0..6).all(|face_index| {
            let start = face_index * 9;
            let face = &self.facelets[start..start + 9];
            face.iter().all(|color| *color == face[0])
        })
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn apply_move(&mut self, command: RotationCommand) -> bool {
        if !self.apply_raw(command) {
            return false;
        }
        if self.history.len() < 512 {
            self.history.push(command);
        }
        self.redo.clear();
        self.sync_facelets();
        true
    }

    pub fn undo(&mut self) -> bool {
        let Some(command) = self.history.pop() else {
            return false;
        };
        if !self.apply_raw(command.inverse()) {
            return false;
        }
        if self.redo.len() < 512 {
            self.redo.push(command);
        }
        self.sync_facelets();
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(command) = self.redo.pop() else {
            return false;
        };
        if !self.apply_raw(command) {
            return false;
        }
        if self.history.len() < 512 {
            self.history.push(command);
        }
        self.sync_facelets();
        true
    }

    pub fn apply_raw(&mut self, command: RotationCommand) -> bool {
        let axis = command.axis.index();
        let rotation = rotation_matrix(command.axis, command.direction);
        let mut changed = false;
        for cubie in &mut self.cubies {
            if cubie.coordinate[axis] == command.layer {
                cubie.coordinate =
                    rotate_coordinate(cubie.coordinate, command.axis, command.direction);
                cubie.orientation = multiply_i8(rotation, cubie.orientation);
                changed = true;
            }
        }
        changed
    }

    fn sync_facelets(&mut self) {
        self.facelets.fill(6);
        for cubie in &self.cubies {
            for (local_face, color) in cubie.face_colors.into_iter().enumerate() {
                if color >= 6 {
                    continue;
                }
                let normal = multiply_vec(cubie.orientation, face_normals()[local_face]);
                let axis = normal.iter().position(|component| *component != 0);
                let Some(axis) = axis else { continue };
                if cubie.coordinate[axis] != normal[axis] {
                    continue;
                }
                let index = facelet_index(normal, cubie.coordinate);
                self.facelets[index] = color;
            }
        }
    }
}

impl Default for CubeState {
    fn default() -> Self {
        Self::new()
    }
}

pub const fn face_normals() -> [[i8; 3]; 6] {
    [
        [0, 0, 1],
        [0, 0, -1],
        [-1, 0, 0],
        [1, 0, 0],
        [0, 1, 0],
        [0, -1, 0],
    ]
}

fn identity_orientation() -> [[i8; 3]; 3] {
    [[1, 0, 0], [0, 1, 0], [0, 0, 1]]
}

fn initial_face_colors(coordinate: [i8; 3]) -> [u8; 6] {
    let mut colors = [6_u8; 6];
    if coordinate[2] == 1 {
        colors[0] = 2;
    }
    if coordinate[2] == -1 {
        colors[1] = 5;
    }
    if coordinate[0] == -1 {
        colors[2] = 0;
    }
    if coordinate[0] == 1 {
        colors[3] = 3;
    }
    if coordinate[1] == 1 {
        colors[4] = 4;
    }
    if coordinate[1] == -1 {
        colors[5] = 1;
    }
    colors
}

fn rotation_matrix(axis: Axis, direction: i8) -> [[i8; 3]; 3] {
    match (axis, direction) {
        (Axis::X, 1) => [[1, 0, 0], [0, 0, -1], [0, 1, 0]],
        (Axis::X, -1) => [[1, 0, 0], [0, 0, 1], [0, -1, 0]],
        (Axis::Y, 1) => [[0, 0, 1], [0, 1, 0], [-1, 0, 0]],
        (Axis::Y, -1) => [[0, 0, -1], [0, 1, 0], [1, 0, 0]],
        (Axis::Z, 1) => [[0, -1, 0], [1, 0, 0], [0, 0, 1]],
        (Axis::Z, -1) => [[0, 1, 0], [-1, 0, 0], [0, 0, 1]],
        _ => identity_orientation(),
    }
}

fn rotate_coordinate(coordinate: [i8; 3], axis: Axis, direction: i8) -> [i8; 3] {
    let [x, y, z] = coordinate;
    match (axis, direction) {
        (Axis::X, 1) => [x, -z, y],
        (Axis::X, -1) => [x, z, -y],
        (Axis::Y, 1) => [z, y, -x],
        (Axis::Y, -1) => [-z, y, x],
        (Axis::Z, 1) => [-y, x, z],
        (Axis::Z, -1) => [y, -x, z],
        _ => coordinate,
    }
}

fn multiply_i8(a: [[i8; 3]; 3], b: [[i8; 3]; 3]) -> [[i8; 3]; 3] {
    let mut result = [[0; 3]; 3];
    for row in 0..3 {
        for column in 0..3 {
            result[row][column] = (0..3).map(|index| a[row][index] * b[index][column]).sum();
        }
    }
    result
}

fn multiply_vec(matrix: [[i8; 3]; 3], vector: [i8; 3]) -> [i8; 3] {
    [
        matrix[0][0] * vector[0] + matrix[0][1] * vector[1] + matrix[0][2] * vector[2],
        matrix[1][0] * vector[0] + matrix[1][1] * vector[1] + matrix[1][2] * vector[2],
        matrix[2][0] * vector[0] + matrix[2][1] * vector[1] + matrix[2][2] * vector[2],
    ]
}

fn facelet_index(normal: [i8; 3], coordinate: [i8; 3]) -> usize {
    let face = match normal {
        [0, 0, 1] => 0,
        [0, 0, -1] => 1,
        [-1, 0, 0] => 2,
        [1, 0, 0] => 3,
        [0, 1, 0] => 4,
        [0, -1, 0] => 5,
        _ => 0,
    };
    let (row, column) = match face {
        0 => (1 - coordinate[1], coordinate[0] + 1),
        1 => (1 - coordinate[1], 1 - coordinate[0]),
        2 => (1 - coordinate[1], 1 - coordinate[2]),
        3 => (1 - coordinate[1], coordinate[2] + 1),
        4 => (coordinate[2] + 1, coordinate[0] + 1),
        _ => (1 - coordinate[2], coordinate[0] + 1),
    };
    face * 9 + (row as usize) * 3 + column as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solved_state_has_six_uniform_faces() {
        let state = CubeState::new();
        assert!(state.is_solved());
        assert_eq!(
            state.facelets().iter().filter(|color| **color < 6).count(),
            54
        );
    }

    #[test]
    fn four_quarter_turns_restore_cubies_and_facelets() {
        for axis in [Axis::X, Axis::Y, Axis::Z] {
            for layer in -1..=1 {
                let mut state = CubeState::new();
                let initial = state.clone();
                let command = RotationCommand::new(axis, layer, 1).unwrap();
                for _ in 0..4 {
                    assert!(state.apply_move(command));
                }
                assert_eq!(state, initial);
            }
        }
    }

    #[test]
    fn undo_and_redo_restore_the_exact_state() {
        let mut state = CubeState::new();
        let initial = state.clone();
        let command = RotationCommand::new(Axis::X, 1, 1).unwrap();
        assert!(state.apply_move(command));
        assert!(!state.is_solved());
        assert!(state.undo());
        assert_eq!(state, initial);
        assert!(state.redo());
        assert!(!state.is_solved());
    }
}
