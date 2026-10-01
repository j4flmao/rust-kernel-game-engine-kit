#include "rke_native.h"
#include <stddef.h>

void rke_batch_transform_xyz(const float *input_xyz,
                             float *output_xyz,
                             uint32_t count,
                             const float *matrix_3x4) {
    for (size_t i = 0; i < count; ++i) {
        const float x = input_xyz[i * 3u + 0u];
        const float y = input_xyz[i * 3u + 1u];
        const float z = input_xyz[i * 3u + 2u];
        output_xyz[i * 3u + 0u] = matrix_3x4[0] * x + matrix_3x4[1] * y + matrix_3x4[2] * z + matrix_3x4[3];
        output_xyz[i * 3u + 1u] = matrix_3x4[4] * x + matrix_3x4[5] * y + matrix_3x4[6] * z + matrix_3x4[7];
        output_xyz[i * 3u + 2u] = matrix_3x4[8] * x + matrix_3x4[9] * y + matrix_3x4[10] * z + matrix_3x4[11];
    }
}

void rke_batch_cull_spheres(const float *spheres_xyzw,
                            uint32_t count,
                            const float *planes_xyzd,
                            uint8_t *visible) {
    for (size_t object = 0; object < count; ++object) {
        const float *sphere = spheres_xyzw + object * 4u;
        uint8_t inside = 1u;
        for (size_t plane = 0; plane < 6u; ++plane) {
            const float *p = planes_xyzd + plane * 4u;
            const float distance = p[0] * sphere[0] + p[1] * sphere[1] +
                                   p[2] * sphere[2] + p[3];
            if (distance < -sphere[3]) {
                inside = 0u;
                break;
            }
        }
        visible[object] = inside;
    }
}

void rke_generate_vertex_normals(const float *positions_xyz,
                                 const uint32_t *indices,
                                 uint32_t vertex_count,
                                 uint32_t index_count,
                                 float *normals_xyz) {
    for (size_t vertex = 0; vertex < (size_t)vertex_count * 3u; ++vertex) normals_xyz[vertex] = 0.0f;
    for (size_t triangle = 0; triangle + 2u < index_count; triangle += 3u) {
        const size_t ia = (size_t)indices[triangle] * 3u;
        const size_t ib = (size_t)indices[triangle + 1u] * 3u;
        const size_t ic = (size_t)indices[triangle + 2u] * 3u;
        const float abx = positions_xyz[ib] - positions_xyz[ia];
        const float aby = positions_xyz[ib + 1u] - positions_xyz[ia + 1u];
        const float abz = positions_xyz[ib + 2u] - positions_xyz[ia + 2u];
        const float acx = positions_xyz[ic] - positions_xyz[ia];
        const float acy = positions_xyz[ic + 1u] - positions_xyz[ia + 1u];
        const float acz = positions_xyz[ic + 2u] - positions_xyz[ia + 2u];
        const float nx = aby * acz - abz * acy;
        const float ny = abz * acx - abx * acz;
        const float nz = abx * acy - aby * acx;
        normals_xyz[ia] += nx; normals_xyz[ia + 1u] += ny; normals_xyz[ia + 2u] += nz;
        normals_xyz[ib] += nx; normals_xyz[ib + 1u] += ny; normals_xyz[ib + 2u] += nz;
        normals_xyz[ic] += nx; normals_xyz[ic + 1u] += ny; normals_xyz[ic + 2u] += nz;
    }
    for (size_t vertex = 0; vertex < vertex_count; ++vertex) {
        const size_t base = vertex * 3u;
        const float length = normals_xyz[base] * normals_xyz[base] +
                             normals_xyz[base + 1u] * normals_xyz[base + 1u] +
                             normals_xyz[base + 2u] * normals_xyz[base + 2u];
        if (length > 0.0f) {
            const float inverse = 1.0f / sqrtf(length);
            normals_xyz[base] *= inverse;
            normals_xyz[base + 1u] *= inverse;
            normals_xyz[base + 2u] *= inverse;
        }
    }
}

void rke_calculate_aabb(const float *positions_xyz,
                        uint32_t vertex_count,
                        float *min_xyz,
                        float *max_xyz) {
    if (vertex_count == 0u) return;
    min_xyz[0] = max_xyz[0] = positions_xyz[0];
    min_xyz[1] = max_xyz[1] = positions_xyz[1];
    min_xyz[2] = max_xyz[2] = positions_xyz[2];
    for (size_t vertex = 1u; vertex < vertex_count; ++vertex) {
        const size_t base = vertex * 3u;
        for (size_t axis = 0u; axis < 3u; ++axis) {
            const float value = positions_xyz[base + axis];
            if (value < min_xyz[axis]) min_xyz[axis] = value;
            if (value > max_xyz[axis]) max_xyz[axis] = value;
        }
    }
}
