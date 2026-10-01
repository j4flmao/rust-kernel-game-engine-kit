#version 450

layout(location = 0) flat in uint in_material;
layout(location = 1) flat in uint in_face;
layout(location = 2) in vec3 in_local_position;
layout(location = 0) out vec4 out_color;

const vec3 PALETTE[6] = vec3[](
    vec3(0.92, 0.12, 0.12),
    vec3(0.98, 0.90, 0.16),
    vec3(0.95, 0.95, 0.95),
    vec3(0.12, 0.32, 0.92),
    vec3(0.95, 0.48, 0.08),
    vec3(0.10, 0.68, 0.28)
);

const vec3 FACE_NORMALS[6] = vec3[](
    vec3(0.0, 0.0, 1.0),
    vec3(0.0, 0.0, -1.0),
    vec3(-1.0, 0.0, 0.0),
    vec3(1.0, 0.0, 0.0),
    vec3(0.0, 1.0, 0.0),
    vec3(0.0, -1.0, 0.0)
);

const vec3 LIGHT_DIRECTION = normalize(vec3(-0.45, 0.75, 0.55));

void main() {
    // Voxel mode has solid terrain materials, not rounded Rubik stickers.
    if ((in_material & 0x80000000u) != 0u) {
        const vec3 terrain[3] = vec3[](vec3(0.22,0.65,0.12), vec3(0.42,0.25,0.11), vec3(0.48,0.50,0.54));
        float shade = 0.35 + 0.65 * max(dot(FACE_NORMALS[in_face], LIGHT_DIRECTION), 0.0);
        out_color = vec4(terrain[min(in_material & 255u, 2u)] * shade, 1.0);
        return;
    }
    uint color_index = (in_material >> (in_face * 3u)) & 7u;
    // Back-face culling is intentionally disabled in the native bring-up
    // pipeline. Let the depth buffer choose the exterior surface instead of
    // relying on a winding convention that differs between Vulkan viewport
    // conventions and the hand-authored cube triangles.
    if (color_index >= 6u) {
        discard;
    }
    vec3 color = PALETTE[color_index];

    // Round the four corners of each sticker slightly.  The discarded edge
    // exposes the dark cubie seam, giving the same separated 3x3 facelet
    // silhouette as a physical cube without adding another mesh stream.
    vec2 sticker_uv;
    if (in_face == 0u) {
        sticker_uv = vec2(in_local_position.x, in_local_position.y);
    } else if (in_face == 1u) {
        sticker_uv = vec2(-in_local_position.x, in_local_position.y);
    } else if (in_face == 2u) {
        sticker_uv = vec2(in_local_position.z, in_local_position.y);
    } else if (in_face == 3u) {
        sticker_uv = vec2(-in_local_position.z, in_local_position.y);
    } else if (in_face == 4u) {
        sticker_uv = vec2(in_local_position.x, -in_local_position.z);
    } else {
        sticker_uv = vec2(in_local_position.x, in_local_position.z);
    }
    vec2 corner = max(abs(sticker_uv) - vec2(0.80), vec2(0.0));
    if (length(corner) > 0.16) {
        discard;
    }
    vec3 normal = FACE_NORMALS[in_face];
    float diffuse = max(dot(normal, LIGHT_DIRECTION), 0.0);
    float ambient = 0.30;
    float shade = ambient + diffuse * 0.70;
    out_color = vec4(color * shade, 1.0);
}
