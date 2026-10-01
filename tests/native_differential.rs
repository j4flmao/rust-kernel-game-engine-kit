use rust_kernel_game_engine_kit::native_accel::{self as selected, reference};

const PLANES: [f32; 24] = [
    1., 0., 0., 1., -1., 0., 0., 1., 0., 1., 0., 1., 0., -1., 0., 1., 0., 0., 1., 1., 0., 0., -1.,
    1.,
];

fn close(actual: &[f32], expected: &[f32]) {
    assert_eq!(actual.len(), expected.len());
    for (&a, &b) in actual.iter().zip(expected) {
        assert!((a - b).abs() <= 2e-5 * (1.0 + b.abs()), "{a} != {b}");
    }
}

#[test]
fn backend_matches_target_and_feature() {
    assert_eq!(
        selected::backend_name(),
        if cfg!(all(unix, feature = "native-accel")) {
            "c"
        } else {
            "rust-fallback"
        }
    );
}

#[test]
fn seeded_batches_match_scalar_reference() {
    let mut seed = 0x4d595df4_u32;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        (seed % 4096) as f32 / 256.0 - 8.0
    };
    for count in [0, 1, 2, 3, 7, 15, 16, 17, 31, 63, 64, 65, 257, 4097] {
        for _ in 0..8 {
            let points: Vec<_> = (0..count * 3).map(|_| next()).collect();
            let matrix = core::array::from_fn(|_| next());
            let mut actual = vec![1234.; points.len() + 2];
            let mut expected = vec![0.; points.len()];
            selected::batch_transform_xyz(&points, &mut actual[1..points.len() + 1], &matrix);
            reference::batch_transform_xyz(&points, &mut expected, &matrix);
            close(&actual[1..points.len() + 1], &expected);
            assert_eq!(actual[0], 1234.);
            assert_eq!(actual[points.len() + 1], 1234.);
            assert_eq!(
                selected::calculate_aabb(&points),
                reference::calculate_aabb(&points)
            );

            let spheres: Vec<_> = (0..count)
                .flat_map(|_| [next(), next(), next(), next().abs()])
                .collect();
            let mut visible = vec![0; count];
            let mut baseline = vec![0; count];
            selected::batch_cull_spheres(&spheres, &PLANES, &mut visible);
            reference::batch_cull_spheres(&spheres, &PLANES, &mut baseline);
            assert_eq!(visible, baseline);

            let indices: Vec<_> = (0..(count / 3 * 3) as u32).collect();
            let mut normals = vec![0.; points.len()];
            let mut expected_normals = vec![0.; points.len()];
            selected::generate_vertex_normals(&points, &indices, &mut normals);
            reference::generate_vertex_normals(&points, &indices, &mut expected_normals);
            close(&normals, &expected_normals);
        }
    }
}

#[test]
fn boundaries_and_degenerate_triangles() {
    let mut visible = [0; 3];
    selected::batch_cull_spheres(
        &[1.25, 0., 0., 0.25, 1.251, 0., 0., 0.25, 0., 0., 0., 0.],
        &PLANES,
        &mut visible,
    );
    assert_eq!(visible, [1, 0, 1]);
    let mut normals = [99.; 9];
    selected::generate_vertex_normals(&[0.; 9], &[0, 0, 0, 0, 1, 2], &mut normals);
    assert_eq!(normals, [0.; 9]);
    assert_eq!(selected::calculate_aabb(&[]), None);
}

#[test]
fn malformed_inputs_panic_before_ffi() {
    use std::panic::catch_unwind;
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert!(catch_unwind(|| selected::calculate_aabb(&[bad, 0., 0.])).is_err());
        assert!(catch_unwind(|| selected::batch_transform_xyz(
            &[bad, 0., 0.],
            &mut [0.; 3],
            &[0.; 12]
        ))
        .is_err());
        assert!(
            catch_unwind(|| selected::batch_transform_xyz(&[0.; 3], &mut [0.; 3], &[bad; 12]))
                .is_err()
        );
        assert!(
            catch_unwind(|| selected::batch_cull_spheres(&[0.; 4], &[bad; 24], &mut [0])).is_err()
        );
        assert!(catch_unwind(|| selected::generate_vertex_normals(
            &[bad, 0., 0.],
            &[],
            &mut [0.; 3]
        ))
        .is_err());
    }
    assert!(
        catch_unwind(|| selected::batch_transform_xyz(&[0.; 2], &mut [0.; 2], &[0.; 12])).is_err()
    );
    assert!(catch_unwind(|| selected::batch_transform_xyz(&[0.; 3], &mut [], &[0.; 12])).is_err());
    assert!(catch_unwind(|| selected::batch_cull_spheres(&[0.; 3], &PLANES, &mut [0])).is_err());
    assert!(
        catch_unwind(|| selected::batch_cull_spheres(&[0., 0., 0., -1.], &PLANES, &mut [0]))
            .is_err()
    );
    assert!(catch_unwind(|| selected::generate_vertex_normals(
        &[0.; 3],
        &[u32::MAX, 0, 0],
        &mut [0.; 3]
    ))
    .is_err());
    assert!(
        catch_unwind(|| selected::generate_vertex_normals(&[0.; 3], &[0, 0], &mut [0.; 3]))
            .is_err()
    );
    assert!(catch_unwind(|| selected::calculate_aabb(&[0.; 2])).is_err());
}
