#version 450
#extension GL_ARB_separate_shader_objects : enable

layout (location = 0) in vec2 in_pos;
layout (location = 1) in vec2 in_uv;
layout (location = 2) in vec4 in_col;

layout (push_constant) uniform PushConstants {
	mat4 matrix;
} pc;

layout (location = 0) out vec2 pass_uv;
layout (location = 1) out vec4 pass_col;

void main() {
    pass_uv = in_uv;
    pass_col = in_col;
    gl_Position = pc.matrix * vec4(in_pos.xy, 0.0f, 1.0f);
}
