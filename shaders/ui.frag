#version 450

layout(location = 0) in float in_opacity;
layout(location = 1) flat in vec4 in_clip;
layout(location = 2) flat in vec4 in_color;
layout(location = 0) out vec4 out_color;

layout(push_constant) uniform UiViewport {
    vec2 size;
} viewport;

void main() {
    float top = viewport.size.y - (in_clip.y + in_clip.w);
    float bottom = viewport.size.y - in_clip.y;
    if (gl_FragCoord.x < in_clip.x || gl_FragCoord.x > in_clip.x + in_clip.z ||
        gl_FragCoord.y < top || gl_FragCoord.y > bottom) {
        discard;
    }
    out_color = vec4(in_color.rgb, clamp(in_color.a * in_opacity, 0.0, 1.0));
}
