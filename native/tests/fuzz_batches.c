#include "rke_native.h"
#include <assert.h>
#include <stddef.h>
#include <stdlib.h>

/* Only valid C ABI inputs reach the kernels. Rust tests exercise invalid slices. */
int LLVMFuzzerTestOneInput(const uint8_t *data, size_t size) {
    if (size == 0) return 0;
    const uint32_t count = data[0]; /* Includes zero, small and non-SIMD multiples. */
    const size_t elements = (size_t)count * 3u;
    float *points = malloc((elements + 1u) * sizeof(float));
    float *output = malloc((elements + 2u) * sizeof(float));
    float *normals = malloc((elements + 2u) * sizeof(float));
    float *spheres = malloc(((size_t)count * 4u + 1u) * sizeof(float));
    uint8_t *visible = malloc((size_t)count + 2u);
    uint32_t *indices = malloc(((size_t)count + 1u) * sizeof(uint32_t));
    assert(points && output && normals && spheres && visible && indices);
    for (size_t i = 0; i < elements; ++i)
        points[i] = ((float)data[i % size] - 128.0f) / 16.0f;
    for (size_t i = 0; i < count; ++i) {
        indices[i] = (uint32_t)i;
        for (size_t axis = 0; axis < 3u; ++axis)
            spheres[i * 4u + axis] = points[i * 3u + axis];
        spheres[i * 4u + 3u] = 0.25f;
    }
    const float identity[12] = {1,0,0,0, 0,1,0,0, 0,0,1,0};
    const float planes[24] = {1,0,0,1, -1,0,0,1, 0,1,0,1, 0,-1,0,1, 0,0,1,1, 0,0,-1,1};
    output[0] = output[elements + 1u] = 12345.0f;
    normals[0] = normals[elements + 1u] = 12345.0f;
    visible[0] = visible[count + 1u] = 123u;
    rke_batch_transform_xyz(points, output + 1, count, identity);
    for (size_t i = 0; i < elements; ++i) assert(output[i + 1u] == points[i]);
    rke_batch_cull_spheres(spheres, count, planes, visible + 1);
    for (size_t i = 0; i < count; ++i) {
        const float *p = points + i * 3u;
        const int expected = fabsf(p[0]) <= 1.25f && fabsf(p[1]) <= 1.25f && fabsf(p[2]) <= 1.25f;
        assert(visible[i + 1u] == (uint8_t)expected);
    }
    rke_generate_vertex_normals(points, indices, count, count / 3u * 3u, normals + 1);
    for (size_t i = 0; i < elements; ++i) assert(isfinite(normals[i + 1u]));
    float lo[3] = {12345,12345,12345}, hi[3] = {12345,12345,12345};
    rke_calculate_aabb(points, count, lo, hi);
    if (count == 0) {
        for (size_t axis = 0; axis < 3u; ++axis) assert(lo[axis] == 12345 && hi[axis] == 12345);
    } else {
        for (size_t i = 0; i < count; ++i)
            for (size_t axis = 0; axis < 3u; ++axis)
                assert(points[i * 3u + axis] >= lo[axis] && points[i * 3u + axis] <= hi[axis]);
    }
    assert(output[0] == 12345 && output[elements + 1u] == 12345);
    assert(normals[0] == 12345 && normals[elements + 1u] == 12345);
    assert(visible[0] == 123u && visible[count + 1u] == 123u);
    free(indices); free(visible); free(spheres); free(normals); free(output); free(points);
    return 0;
}

#ifdef RKE_STANDALONE
int main(void) {
    uint8_t data[1024];
    uint32_t seed = 0x4d595df4u;
    for (unsigned run = 0; run < 4096; ++run) {
        for (size_t i = 0; i < sizeof(data); ++i) {
            seed ^= seed << 13; seed ^= seed >> 17; seed ^= seed << 5;
            data[i] = (uint8_t)seed;
        }
        data[0] = (uint8_t)run;
        LLVMFuzzerTestOneInput(data, sizeof(data));
    }
    return 0;
}
#endif
