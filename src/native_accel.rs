//! Optional batch acceleration boundary.
//!
//! The public API is safe and checks slice sizes before crossing the C ABI.
//! The feature is intentionally opt-in so the engine always has a pure-Rust
//! fallback and does not require a native toolchain for normal builds.

#[cfg(all(target_family = "unix", feature = "native-accel"))]
unsafe extern "C" {
    fn rke_batch_transform_xyz(
        input_xyz: *const f32,
        output_xyz: *mut f32,
        count: u32,
        matrix_3x4: *const f32,
    );
    fn rke_batch_cull_spheres(
        spheres_xyzw: *const f32,
        count: u32,
        planes_xyzd: *const f32,
        visible: *mut u8,
    );
    fn rke_generate_vertex_normals(
        positions_xyz: *const f32,
        indices: *const u32,
        vertex_count: u32,
        index_count: u32,
        normals_xyz: *mut f32,
    );
    fn rke_calculate_aabb(
        positions_xyz: *const f32,
        vertex_count: u32,
        min_xyz: *mut f32,
        max_xyz: *mut f32,
    );
}

pub fn batch_transform_xyz(input: &[f32], output: &mut [f32], matrix_3x4: &[f32; 12]) {
    assert!(input.len().is_multiple_of(3));
    assert_eq!(input.len(), output.len());
    let count = u32::try_from(input.len() / 3).expect("native batch is too large");

    #[cfg(all(target_family = "unix", feature = "native-accel"))]
    {
        // SAFETY: lengths and pointers are validated above; the C function
        // writes exactly `input.len()` output elements.
        unsafe {
            rke_batch_transform_xyz(
                input.as_ptr(),
                output.as_mut_ptr(),
                count,
                matrix_3x4.as_ptr(),
            );
        }
    }

    #[cfg(not(all(target_family = "unix", feature = "native-accel")))]
    {
        for index in 0..count as usize {
            let base = index * 3;
            let x = input[base];
            let y = input[base + 1];
            let z = input[base + 2];
            output[base] =
                matrix_3x4[0] * x + matrix_3x4[1] * y + matrix_3x4[2] * z + matrix_3x4[3];
            output[base + 1] =
                matrix_3x4[4] * x + matrix_3x4[5] * y + matrix_3x4[6] * z + matrix_3x4[7];
            output[base + 2] =
                matrix_3x4[8] * x + matrix_3x4[9] * y + matrix_3x4[10] * z + matrix_3x4[11];
        }
    }
}

/// Returns the implementation selected for the current target.
pub const fn backend_name() -> &'static str {
    if cfg!(all(target_family = "unix", feature = "native-accel")) {
        "c"
    } else {
        "rust-fallback"
    }
}

pub fn batch_cull_spheres(spheres_xyzw: &[f32], planes_xyzd: &[f32; 24], visible: &mut [u8]) {
    assert!(spheres_xyzw.len().is_multiple_of(4));
    assert_eq!(spheres_xyzw.len() / 4, visible.len());
    let count = u32::try_from(visible.len()).expect("native culling batch is too large");

    #[cfg(all(target_family = "unix", feature = "native-accel"))]
    {
        // SAFETY: all slices are validated and C writes one byte per sphere.
        unsafe {
            rke_batch_cull_spheres(
                spheres_xyzw.as_ptr(),
                count,
                planes_xyzd.as_ptr(),
                visible.as_mut_ptr(),
            );
        }
    }

    #[cfg(not(all(target_family = "unix", feature = "native-accel")))]
    {
        rust_batch_cull_spheres(spheres_xyzw, planes_xyzd, visible, count);
    }
}

#[allow(unused_variables)]
pub fn generate_vertex_normals(positions_xyz: &[f32], indices: &[u32], normals_xyz: &mut [f32]) {
    assert!(positions_xyz.len().is_multiple_of(3));
    assert!(indices.len().is_multiple_of(3));
    assert_eq!(positions_xyz.len(), normals_xyz.len());
    assert!(indices
        .iter()
        .all(|index| *index < positions_xyz.len() as u32 / 3));
    let vertex_count = u32::try_from(normals_xyz.len() / 3).expect("mesh is too large");
    let index_count = u32::try_from(indices.len()).expect("index buffer is too large");

    #[cfg(all(target_family = "unix", feature = "native-accel"))]
    unsafe {
        rke_generate_vertex_normals(
            positions_xyz.as_ptr(),
            indices.as_ptr(),
            vertex_count,
            index_count,
            normals_xyz.as_mut_ptr(),
        );
    }
    #[cfg(not(all(target_family = "unix", feature = "native-accel")))]
    rust_generate_vertex_normals(positions_xyz, indices, normals_xyz);
}

#[allow(unused_variables, unused_assignments)]
pub fn calculate_aabb(positions_xyz: &[f32]) -> Option<([f32; 3], [f32; 3])> {
    if positions_xyz.is_empty() {
        return None;
    }
    assert!(positions_xyz.len().is_multiple_of(3));
    let count = u32::try_from(positions_xyz.len() / 3).expect("mesh is too large");
    let mut min_xyz = [0.0; 3];
    let mut max_xyz = [0.0; 3];
    #[cfg(all(target_family = "unix", feature = "native-accel"))]
    unsafe {
        rke_calculate_aabb(
            positions_xyz.as_ptr(),
            count,
            min_xyz.as_mut_ptr(),
            max_xyz.as_mut_ptr(),
        );
    }
    #[cfg(not(all(target_family = "unix", feature = "native-accel")))]
    {
        min_xyz = [positions_xyz[0], positions_xyz[1], positions_xyz[2]];
        max_xyz = min_xyz;
        for point in positions_xyz.chunks_exact(3).skip(1) {
            for axis in 0..3 {
                min_xyz[axis] = min_xyz[axis].min(point[axis]);
                max_xyz[axis] = max_xyz[axis].max(point[axis]);
            }
        }
    }
    Some((min_xyz, max_xyz))
}

#[cfg(not(all(target_family = "unix", feature = "native-accel")))]
fn rust_generate_vertex_normals(positions: &[f32], indices: &[u32], normals: &mut [f32]) {
    normals.fill(0.0);
    for triangle in indices.chunks_exact(3) {
        let a = triangle[0] as usize * 3;
        let b = triangle[1] as usize * 3;
        let c = triangle[2] as usize * 3;
        let ab = [
            positions[b] - positions[a],
            positions[b + 1] - positions[a + 1],
            positions[b + 2] - positions[a + 2],
        ];
        let ac = [
            positions[c] - positions[a],
            positions[c + 1] - positions[a + 1],
            positions[c + 2] - positions[a + 2],
        ];
        let normal = [
            ab[1] * ac[2] - ab[2] * ac[1],
            ab[2] * ac[0] - ab[0] * ac[2],
            ab[0] * ac[1] - ab[1] * ac[0],
        ];
        for index in [a, b, c] {
            for axis in 0..3 {
                normals[index + axis] += normal[axis];
            }
        }
    }
    for normal in normals.chunks_exact_mut(3) {
        let length = normal.iter().map(|value| value * value).sum::<f32>().sqrt();
        if length > 0.0 {
            for value in normal {
                *value /= length;
            }
        }
    }
}

fn rust_batch_cull_spheres(
    spheres_xyzw: &[f32],
    planes_xyzd: &[f32; 24],
    visible: &mut [u8],
    count: u32,
) {
    for object in 0..count as usize {
        let sphere = &spheres_xyzw[object * 4..object * 4 + 4];
        visible[object] = planes_xyzd.chunks_exact(4).all(|plane| {
            plane[0] * sphere[0] + plane[1] * sphere[1] + plane[2] * sphere[2] + plane[3]
                >= -sphere[3]
        }) as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::{
        backend_name, batch_cull_spheres, batch_transform_xyz, calculate_aabb,
        generate_vertex_normals,
    };

    #[test]
    fn batch_transform_matches_affine_contract() {
        let input = [1.0, 2.0, 3.0, -1.0, 0.0, 2.0];
        let mut output = [0.0; 6];
        batch_transform_xyz(
            &input,
            &mut output,
            &[2.0, 0.0, 0.0, 1.0, 0.0, 3.0, 0.0, -2.0, 0.0, 0.0, 4.0, 5.0],
        );
        assert_eq!(output, [3.0, 4.0, 17.0, -1.0, -2.0, 13.0]);
    }

    #[test]
    fn batch_culling_rejects_spheres_outside_any_plane() {
        let planes = [
            1.0, 0.0, 0.0, 1.0, -1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, -1.0, 0.0, 1.0, 0.0,
            0.0, 1.0, 1.0, 0.0, 0.0, -1.0, 1.0,
        ];
        let spheres = [0.0, 0.0, 0.0, 0.1, 3.0, 0.0, 0.0, 0.1];
        let mut visible = [0; 2];
        batch_cull_spheres(&spheres, &planes, &mut visible);
        assert_eq!(visible, [1, 0]);
    }

    #[test]
    fn backend_is_explicit() {
        assert!(!backend_name().is_empty());
    }

    #[test]
    fn mesh_helpers_produce_normals_and_bounds() {
        let positions = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let indices = [0, 1, 2];
        let mut normals = [0.0; 9];
        generate_vertex_normals(&positions, &indices, &mut normals);
        assert!(normals.chunks_exact(3).all(|normal| normal[2] > 0.99));
        assert_eq!(
            calculate_aabb(&positions),
            Some(([0.0, 0.0, 0.0], [1.0, 1.0, 0.0]))
        );
    }
}
