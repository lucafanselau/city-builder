#version 450
#extension GL_ARB_separate_shader_objects : enable

layout (location = 0) in vec3 pass_normal;
layout (location = 1) in vec3 pass_fragment_position;

layout(push_constant, std430) uniform Material {
    layout (offset = 64) vec4 ambient;
    layout (offset = 80) vec4 diffuse;
    layout (offset = 96) vec4 specular;
    layout (offset = 112) float shininess;
} material;

layout(binding = 1) uniform Light {
    vec3 light_position;
    vec3 view_position;
} light;

layout(location = 0) out vec4 out_color;

void main() {
    // ambient
    vec3 ambient = 0.1 * vec3(material.ambient);
    // diffuse
    vec3 light_dir = normalize(light.light_position - pass_fragment_position);
    vec3 normal = normalize(pass_normal);
    float diff = max(dot(light_dir, normal), 0.0);
    vec3 diffuse = diff * vec3(material.diffuse);
    // specular
    // blinn-phong
    // vec3 view_dir = normalize(light.view_position - pass_fragment_position);
    // vec3 reflect_dir = reflect(-light_dir, normal);
    // vec3 halfway_dir = normalize(light_dir + view_dir);  
    // float spec = pow(max(dot(normal, halfway_dir), 0.0), material.shininess);
    // vec3 specular = material.specular * spec; // assuming bright white light color
    // just blinn
    vec3 view_dir = normalize(light.view_position - pass_fragment_position);
    vec3 reflect_dir = reflect(-light_dir, normal);  
    float spec = pow(max(dot(view_dir, reflect_dir), 0.0), material.shininess);
    vec3 specular = spec * vec3(material.specular);
    
    out_color = vec4(ambient + diffuse + specular, material.specular.a);
    // out_color = vec4(0.43, 0.9, 0.8, 1.0);
}
