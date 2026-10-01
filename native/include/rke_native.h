#ifndef RKE_NATIVE_H
#define RKE_NATIVE_H

#include <stdint.h>
#include <math.h>

/* ABI contract: caller owns distinct, non-overlapping buffers of the declared
 * lengths. Inputs must be finite, radii nonnegative, indices < vertex_count,
 * and index_count a multiple of three. Zero counts do not dereference buffers.
 * Rust wrappers validate these preconditions before entering C.
 * Finite inputs can still overflow floating-point arithmetic; no fast-math. */

/* Applies one affine 3x4 matrix to packed xyz input/output vectors. */
void rke_batch_transform_xyz(const float *input_xyz,
                             float *output_xyz,
                             uint32_t count,
                             const float *matrix_3x4);

/* Marks spheres visible when they are not fully outside any frustum plane. */
void rke_batch_cull_spheres(const float *spheres_xyzw,
                            uint32_t count,
                            const float *planes_xyzd,
                            uint8_t *visible);

void rke_generate_vertex_normals(const float *positions_xyz,
                                 const uint32_t *indices,
                                 uint32_t vertex_count,
                                 uint32_t index_count,
                                 float *normals_xyz);

void rke_calculate_aabb(const float *positions_xyz,
                        uint32_t vertex_count,
                        float *min_xyz,
                        float *max_xyz);

#endif
