#version 450
#extension GL_ARB_separate_shader_objects : enable

layout (location = 0) in vec4 vertex_color;

layout (location = 0) out vec4 fragment_color;

void main() {
    fragment_color = vec4(0.75f, 0.23f, 0.75f, 1.0f);
    //vertex_color;
}
