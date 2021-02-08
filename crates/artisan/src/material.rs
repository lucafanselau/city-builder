use bytemuck::{Pod, Zeroable};
use glam::{Vec3, Vec3A};

#[derive(Debug, Clone, Copy)]
#[repr(C)]
#[repr(align(16))]
pub struct SolidMaterial {
    // Vec4 because of alignment
    ambient: Vec3A,
    diffuse: Vec3A,
    specular: Vec3A,
    shininess: f32,
}

unsafe impl Zeroable for SolidMaterial {}
unsafe impl Pod for SolidMaterial {}

#[derive(Debug)]
pub enum MaterialComponent {
    Solid(SolidMaterial),
    // Textured(TextureMaterial),
}

impl MaterialComponent {
    pub const YELLOW_RUBBER: Self = Self::Solid(SolidMaterial {
        ambient: glam::const_vec3a!([0.05, 0.05, 0.0]),
        diffuse: glam::const_vec3a!([0.5, 0.5, 0.4]),
        specular: glam::const_vec3a!([0.7, 0.7, 0.04]),
        shininess: 0.078125,
    });

    pub fn solid(ambient: Vec3, diffuse: Vec3, specular: Vec3, shininess: f32) -> Self {
        Self::Solid(SolidMaterial {
            ambient: ambient.into(),
            diffuse: diffuse.into(),
            specular: specular.into(),
            shininess,
        })
    }
}
