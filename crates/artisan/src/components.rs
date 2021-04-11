use std::ops::Deref;

use crate::mesh::Model;
use app::AssetHandle;

#[derive(Debug)]
pub struct ModelComponent(pub AssetHandle<Model>);

impl Deref for ModelComponent {
    type Target = AssetHandle<Model>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Rotation {
    axis: glam::Vec3,
    /// Angle around axis in radians
    angle: f32,
}

impl Default for Rotation {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl Rotation {
    pub const DEFAULT: Self = Self {
        axis: crate::UP,
        angle: 0.0,
    };

    pub fn new(axis: glam::Vec3, angle: f32) -> Self {
        Self { axis, angle }
    }

    /// Set the rotation's axis.
    pub fn set_axis(&mut self, axis: glam::Vec3) {
        self.axis = axis;
    }

    /// Set the rotation's angle (in radians).
    pub fn set_angle(&mut self, angle: f32) {
        self.angle = angle;
    }

    fn into_quat(self) -> glam::Quat {
        glam::Quat::from_axis_angle(self.axis, self.angle)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Transform {
    pub(crate) pos: glam::Vec3,
    pub(crate) rotation: Rotation,
    pub(crate) scale: glam::Vec3,
}

impl Transform {
    pub const UNIT: Self = Self {
        pos: glam::const_vec3!([0.0, 0.0, 0.0]),
        rotation: Rotation::DEFAULT,
        scale: glam::const_vec3!([1.0, 1.0, 1.0]),
    };

    pub fn new(pos: glam::Vec3, rotation: Rotation, scale: glam::Vec3) -> Self {
        Self {
            pos,
            rotation,
            scale,
        }
    }

    pub fn into_model(self) -> glam::Mat4 {
        glam::Mat4::from_scale_rotation_translation(self.scale, self.rotation.into_quat(), self.pos)
    }

    /// Get a reference to the transform's pos.
    pub fn pos(&self) -> &glam::Vec3 {
        &self.pos
    }

    /// Set the transform's pos.
    pub fn set_pos(&mut self, pos: glam::Vec3) {
        self.pos = pos;
    }

    /// Set the transform's rotation.
    pub fn set_rotation(&mut self, rotation: Rotation) {
        self.rotation = rotation;
    }

    /// Set the transform's scale.
    pub fn set_scale(&mut self, scale: glam::Vec3) {
        self.scale = scale;
    }
}
