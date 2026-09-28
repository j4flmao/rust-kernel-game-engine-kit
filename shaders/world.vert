#version 450

// The instance stream is the stable 80-byte GpuInstanceRecord encoded as
// uint words.  Keeping this ABI explicit prevents Rust padding from becoming
// an accidental shader contract.
layout(set = 0, binding = 0, std430) readonly buffer WorldInstances {
    uint words[];
} world_instances;

layout(push_constant) uniform WorldCamera {
    mat4 view_projection;
} camera;

layout(location = 0) flat out uint out_material;
layout(location = 1) flat out uint out_face;
layout(location = 2) out vec3 out_local_position;

const vec3 CUBE_TRIANGLES[36] = vec3[](
    vec3(-1.0, -1.0,  1.0), vec3( 1.0, -1.0,  1.0), vec3( 1.0,  1.0,  1.0),
    vec3(-1.0, -1.0,  1.0), vec3( 1.0,  1.0,  1.0), vec3(-1.0,  1.0,  1.0),
    vec3( 1.0, -1.0, -1.0), vec3(-1.0, -1.0, -1.0), vec3(-1.0,  1.0, -1.0),
    vec3( 1.0, -1.0, -1.0), vec3(-1.0,  1.0, -1.0), vec3( 1.0,  1.0, -1.0),
    vec3(-1.0, -1.0, -1.0), vec3(-1.0, -1.0,  1.0), vec3(-1.0,  1.0,  1.0),
    vec3(-1.0, -1.0, -1.0), vec3(-1.0,  1.0,  1.0), vec3(-1.0,  1.0, -1.0),
    vec3( 1.0, -1.0,  1.0), vec3( 1.0, -1.0, -1.0), vec3( 1.0,  1.0, -1.0),
    vec3( 1.0, -1.0,  1.0), vec3( 1.0,  1.0, -1.0), vec3( 1.0,  1.0,  1.0),
    vec3(-1.0,  1.0,  1.0), vec3( 1.0,  1.0,  1.0), vec3( 1.0,  1.0, -1.0),
    vec3(-1.0,  1.0,  1.0), vec3( 1.0,  1.0, -1.0), vec3(-1.0,  1.0, -1.0),
    vec3(-1.0, -1.0, -1.0), vec3( 1.0, -1.0, -1.0), vec3( 1.0, -1.0,  1.0),
    vec3(-1.0, -1.0, -1.0), vec3( 1.0, -1.0,  1.0), vec3(-1.0, -1.0,  1.0)
);

void main() {
    uint base = gl_InstanceIndex * 20u;
    vec3 position = CUBE_TRIANGLES[gl_VertexIndex];
    vec3 world_position = vec3(
        uintBitsToFloat(world_instances.words[base + 0u]) * position.x +
            uintBitsToFloat(world_instances.words[base + 1u]) * position.y +
            uintBitsToFloat(world_instances.words[base + 2u]) * position.z +
            uintBitsToFloat(world_instances.words[base + 3u]),
        uintBitsToFloat(world_instances.words[base + 4u]) * position.x +
            uintBitsToFloat(world_instances.words[base + 5u]) * position.y +
            uintBitsToFloat(world_instances.words[base + 6u]) * position.z +
            uintBitsToFloat(world_instances.words[base + 7u]),
        uintBitsToFloat(world_instances.words[base + 8u]) * position.x +
            uintBitsToFloat(world_instances.words[base + 9u]) * position.y +
            uintBitsToFloat(world_instances.words[base + 10u]) * position.z +
            uintBitsToFloat(world_instances.words[base + 11u])
    );
    gl_Position = camera.view_projection * vec4(world_position, 1.0);
    out_material = world_instances.words[base + 19u];
    out_face = gl_VertexIndex / 6u;
    out_local_position = position;
}
