#version 450

layout (location = 0) in vec3 in_pos;

layout (binding = 0) uniform CameraBuffer {
       mat4 view_projection;
} camera;

void main() {
    gl_Position = camera.view_projection * vec4(in_pos, 1.0);
}
