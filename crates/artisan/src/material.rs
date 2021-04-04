use bytemuck::{Pod, Zeroable};
use glam::Vec4;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
#[repr(align(16))]
pub struct SolidMaterial {
    // Vec4 because of alignment
    ambient: Vec4,
    diffuse: Vec4,
    specular: Vec4,
    shininess: f32,
}

unsafe impl Zeroable for SolidMaterial {}
unsafe impl Pod for SolidMaterial {}

#[derive(Debug)]
pub enum Material {
    Solid(SolidMaterial),
    // Textured(TextureMaterial),
}

impl Material {
    pub const YELLOW_RUBBER: Self = Self::Solid(SolidMaterial {
        ambient: glam::const_vec4!([0.05, 0.05, 0.0, 1.0]),
        diffuse: glam::const_vec4!([0.5, 0.5, 0.4, 1.0]),
        specular: glam::const_vec4!([0.7, 0.7, 0.04, 1.0]),
        shininess: 0.078125,
    });

    pub const BRONZE: Self = Self::Solid(SolidMaterial {
        ambient: glam::const_vec4!([1.0, 0.5, 0.31, 1.0]),
        diffuse: glam::const_vec4!([1.0, 0.5, 0.31, 1.0]),
        specular: glam::const_vec4!([0.5, 0.5, 0.5, 1.0]),
        shininess: 32.0,
    });

    pub fn solid(ambient: Vec4, diffuse: Vec4, specular: Vec4, shininess: f32) -> Self {
        Self::Solid(SolidMaterial {
            ambient,
            diffuse,
            specular,
            shininess,
        })
    }
}
