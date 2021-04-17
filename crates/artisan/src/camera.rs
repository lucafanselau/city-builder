use crate::UP;
use app::{App, IntoFunctionSystem, Timing};
use bytemuck::{Pod, Zeroable};
use glam::{Vec3Swizzles, XY};
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
    pub eye: glam::Vec3,
    dir: glam::Vec3,
    // Angles in degrees?
    yaw: f32,
    pitch: f32,
}

const SENSITIVITY: f32 = 0.6;
const MOVEMENT_SENSITIVITY: f32 = 3.0;

impl Camera {
    fn calc(&self, aspect_ratio: f32) -> (glam::Mat4, glam::Mat4) {
        let projection = {
            let initial = glam::Mat4::perspective_rh(45f32.to_radians(), aspect_ratio, 0.1, 100.0);
            //log::info!("initial: {}", initial);
            let mut array = initial.to_cols_array();
            array[5] *= -1.0;
            glam::Mat4::from_cols_array(&array)
        };
        //log::info!("projection: {}", projection);

        let view = glam::Mat4::look_at_rh(self.eye, self.eye + self.dir, UP);

        (projection, view)
    }

    // Loosely taken from: https://antongerdelan.net/opengl/raycasting.html
    pub fn mouse_ray(&self, mouse: &glam::Vec2, aspect_ratio: f32) -> glam::Vec3 {
        let (projection, view) = self.calc(aspect_ratio);

        let ray_nds = glam::vec3(mouse.x * 2.0 - 1.0, mouse.y * 2.0 - 1.0, 1.0);
        let ray_clip = glam::vec4(ray_nds.x, ray_nds.y, -1.0, 1.0);

        let mut ray_eye = projection.inverse() * ray_clip;
        ray_eye.z = -1.0;
        ray_eye.w = 0.0;

        (view.inverse() * ray_eye).truncate().normalize()
    }

    pub fn to_buffer(&self, aspect_ratio: f32) -> CameraBuffer {
        let (projection, view) = self.calc(aspect_ratio);
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

        // log::debug!("CAMERA POS IS: {}", camera.eye);
    }
}

pub(crate) fn init(app: &mut App) {
    app.insert_resource(Camera {
        eye: glam::vec3(0.0, 0.0, 4.0),
        dir: glam::vec3(0.0, 0.0, -1.0),
        yaw: 0.0,
        pitch: 0.0,
    });
    app.add_system(app::stages::UPDATE, camera_system.into_system());
}
