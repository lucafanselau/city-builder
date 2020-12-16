#version 450

layout (location = 0) in vec4 in_pos;

layout (binding = 0) uniform Offset {
       vec4 offset;
} offset;

void main() {
    gl_Position = vec4(in_pos.x + offset.offset[gl_VertexIndex], in_pos.yzw);
}
