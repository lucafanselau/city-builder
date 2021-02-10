use app::{App, IntoFunctionSystem, Timing};
use bytemuck::{Pod, Zeroable};
use glam::{Vec2, XY};
use std::{
    cell::{Ref, RefMut},
    ops::Deref,
};
use window::{events::VirtualKeyCode, input::Input};

/// The struct that can be sent to shaders
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
#[repr(C)]
pub struct CameraBuffer {
    view_projection: glam::Mat4,
}

#[derive(Debug)]
pub struct Camera {
    pub(crate) eye: glam::Vec3,
    dir: glam::Vec3,
    // Angles in degrees?
    yaw: f32,
    pitch: f32,
}

const UP: glam::Vec3 = glam::const_vec3!([0.0, 1.0, 0.0]);
const SENSITIVITY: f32 = 0.6;
const MOVEMENT_SENSITIVITY: f32 = 0.5;

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
    // Rotating
    {
        // log::info!("{:?}", input.mouse_delta);
        // Update Yaw and pitch
        let XY { x, y } = input.mouse_delta.deref();
        camera.yaw -= x * SENSITIVITY;
        camera.pitch += y * SENSITIVITY;

        let pitch = camera.pitch.to_radians();
        let yaw = camera.yaw.to_radians();
        camera.dir = glam::vec3(
            pitch.cos() * yaw.cos(),
            pitch.sin(),
            pitch.cos() * yaw.sin(),
        );
    }
    // Movement
    {
        enum Direction {
            Along,
            Orthogonal,
        }

        // The direction to move in
        let mut delta_dir = glam::vec3(0.0, 0.0, 0.0);
        // The direction that is orthogonal to the forward direction and the Up vector (used to move to the side)
        let right_dir = camera.dir.cross(UP);
        let mut calc_dir = |key: VirtualKeyCode, d: Direction, scalar: f32| {
            if input.is_pressed(key) {
                delta_dir += scalar
                    * match d {
                        Direction::Along => camera.dir,
                        Direction::Orthogonal => right_dir,
                    }
            }
        };
        calc_dir(VirtualKeyCode::W, Direction::Along, 1f32);
        calc_dir(VirtualKeyCode::S, Direction::Along, -1f32);
        calc_dir(VirtualKeyCode::A, Direction::Orthogonal, -1f32);
        calc_dir(VirtualKeyCode::D, Direction::Orthogonal, 1f32);

        if input.is_pressed(VirtualKeyCode::Space) {
            delta_dir += UP
        }
        if input.is_pressed(VirtualKeyCode::LShift) {
            delta_dir -= UP
        }

        camera.eye += MOVEMENT_SENSITIVITY * timing.dt * delta_dir;
    }
}

pub(crate) fn init(app: &mut App) {
    app.insert_resource(Camera {
        eye: glam::vec3(0.0, 0.0, 1.0),
        dir: glam::vec3(0.0, 0.0, -1.0),
        yaw: 0.0,
        pitch: 0.0,
    });
    app.add_system(app::stages::UPDATE, camera_system.into_system());
}
