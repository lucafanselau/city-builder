#version 450
#extension GL_ARB_separate_shader_objects : enable

layout (location = 0) in vec3 in_pos;
layout (location = 1) in vec3 in_normal;

layout (push_constant) uniform PushConstants {
	mat4 transform;
	mat4 view_projection;
} pc;

layout (location = 0) out vec4 vertex_color;

vec3 get_abs(vec3 vector) {
	return vec3(vector.x, abs(vector.y), abs(vector.z));
}

void main() {
    vertex_color = vec4(get_abs(in_normal), 1.0);
    gl_Position = pc.view_projection * pc.transform * vec4(in_pos, 1.0f);
}
