#version 450

// New comment
layout (location = 0) in vec3 in_pos;
layout (location = 1) in vec3 in_normal;

layout (binding = 0) uniform CameraBuffer {
       mat4 view_projection;
} camera;

layout (location = 0) out vec3 pass_normal;

void main() {
    gl_Position = camera.view_projection * vec4(in_pos, 1.0);
    pass_normal = in_normal;
}
