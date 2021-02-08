#version 450
#extension GL_ARB_separate_shader_objects : enable

layout (location = 0) in vec3 pass_normal;

layout(location = 0) out vec4 out_color;

void main() {
    out_color = vec4(((0.5 * pass_normal) + vec3(0.5, 0.5, 0.5)).xyz, 1.0);
}
