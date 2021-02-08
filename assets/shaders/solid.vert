#version 450

layout (location = 0) in vec3 in_pos;
layout (location = 1) in vec3 in_normal;

layout (binding = 0) uniform CameraBuffer {
       mat4 view_projection;
} camera;

layout (location = 0) out vec3 pass_normal;
layout (location = 1) out vec3 pass_fragment_position;

void main() {
    // Pass to fragment shader
    pass_fragment_position = in_pos;
    pass_normal = in_normal;
    // Calculate output position
    gl_Position = camera.view_projection * vec4(in_pos, 1.0);
}
