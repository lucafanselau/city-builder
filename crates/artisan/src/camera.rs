use app::{App, IntoFunctionSystem, Timing};
use bytemuck::{Pod, Zeroable};
use std::cell::{Ref, RefMut};
use window::{events::VirtualKeyCode, input::Input};

/// The struct that can be sent to shaders
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
#[repr(C)]
pub struct CameraBuffer {
    view_projection: glam::Mat4,
}

#[derive(Debug)]
pub struct Camera {
    eye: glam::Vec3,
    dir: glam::Vec3,
}

const UP: glam::Vec3 = glam::const_vec3!([0.0, 1.0, 0.0]);

impl Camera {
    pub fn calc(&self, aspect_ratio: f32) -> CameraBuffer {
        let projection = glam::Mat4::perspective_rh(45f32.to_radians(), aspect_ratio, 0.1, 10.0);
        let view = glam::Mat4::look_at_rh(self.eye, self.eye + self.dir, UP);

        CameraBuffer {
            view_projection: projection * view,
        }
    }
}

fn camera_system(mut camera: RefMut<Camera>, input: Ref<Input>, timing: Ref<Timing>) {
    let mut dir = glam::vec2(0.0, 0.0);
    let mut calc_dir = |key: VirtualKeyCode, d: glam::Vec2| {
        if input.is_pressed(key) {
            dir += d
        }
    };
    calc_dir(VirtualKeyCode::W, glam::vec2(0.0, 1.0));
    calc_dir(VirtualKeyCode::S, glam::vec2(0.0, -1.0));
    calc_dir(VirtualKeyCode::A, glam::vec2(-1.0, 0.0));
    calc_dir(VirtualKeyCode::D, glam::vec2(1.0, 0.0));

    camera.eye += (0.2 * timing.dt * dir).extend(0.0);
}

pub(crate) fn init(app: &mut App) {
    app.insert_resource(Camera {
        eye: glam::vec3(0.0, 0.0, 1.0),
        dir: glam::vec3(0.0, 0.0, -1.0),
    });
    app.add_system(app::stages::UPDATE, camera_system.into_system());
}
