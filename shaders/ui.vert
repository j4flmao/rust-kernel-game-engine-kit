#version 450

// One item is a fixed 80-byte record encoded by UiRenderSnapshot.
// The storage buffer is read as uint words so Rust padding cannot silently
// change the shader ABI.
layout(set = 0, binding = 0, std430) readonly buffer UiItems {
    uint words[];
} ui_items;

layout(push_constant) uniform UiViewport {
    vec2 size;
} viewport;

layout(location = 0) out float out_opacity;
layout(location = 1) flat out vec4 out_clip;
layout(location = 2) flat out vec4 out_color;

void main() {
    uint base = gl_InstanceIndex * 16u;
    vec4 rect = vec4(
        uintBitsToFloat(ui_items.words[base + 0u]),
        uintBitsToFloat(ui_items.words[base + 1u]),
        uintBitsToFloat(ui_items.words[base + 2u]),
        uintBitsToFloat(ui_items.words[base + 3u])
    );
    out_clip = vec4(
        uintBitsToFloat(ui_items.words[base + 4u]),
        uintBitsToFloat(ui_items.words[base + 5u]),
        uintBitsToFloat(ui_items.words[base + 6u]),
        uintBitsToFloat(ui_items.words[base + 7u])
    );
    out_color = vec4(
        uintBitsToFloat(ui_items.words[base + 8u]),
        uintBitsToFloat(ui_items.words[base + 9u]),
        uintBitsToFloat(ui_items.words[base + 10u]),
        uintBitsToFloat(ui_items.words[base + 11u])
    );
    vec2 corners[6] = vec2[](
        vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(1.0, 1.0),
        vec2(0.0, 0.0), vec2(1.0, 1.0), vec2(0.0, 1.0)
    );
    vec2 local = corners[gl_VertexIndex];
    vec2 pixel = rect.xy + local * rect.zw;
    vec2 ndc = pixel / viewport.size * 2.0 - 1.0;
    gl_Position = vec4(ndc.x, -ndc.y, 0.0, 1.0);
    out_opacity = uintBitsToFloat(ui_items.words[base + 12u]);
}
