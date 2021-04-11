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

    pub fn from_color(color: Color) -> Self {
        color.into()
    }
}

impl From<Color> for Material {
    fn from(color: Color) -> Self {
        Self::Solid(SolidMaterial {
            ambient: glam::vec4(0.05, 0.05, 0.05, 1.0),
            diffuse: color.0,
            specular: glam::vec4(0.2, 0.2, 0.2, 1.0),
            shininess: 1.0,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Color(glam::Vec4);

impl Color {
    pub fn new(color: glam::Vec4) -> Self {
        Self(color)
    }

    pub fn from_rgba(r: u8, g: u8, b: u8, a: f32) -> Self {
        Self(glam::vec4(
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
            a,
        ))
    }
}
