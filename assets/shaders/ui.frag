#version 450
#extension GL_ARB_separate_shader_objects : enable

layout (location = 0) in vec2 pass_uv;
layout (location = 1) in vec4 pass_col;

layout(set = 0, binding = 0) uniform texture2D tex;
layout(set = 0, binding = 1) uniform sampler samp;

layout (location = 0) out vec4 fragment_color;

void main() {
    fragment_color = pass_col * texture(sampler2D(tex, samp), pass_uv.st);
}