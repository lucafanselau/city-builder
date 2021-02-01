use app::{App, IntoFunctionSystem, Timing};
use bytemuck::{Pod, Zeroable};
use std::cell::{Ref, RefMut};

///
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

fn camera_system(mut camera: RefMut<Camera>, timing: Ref<Timing>) {
    // Randomly move along plane a bit
    let elapsed = timing.total_elapsed();
    camera.eye = glam::vec3(elapsed.sin() * 0.3, elapsed.sin() * 0.3, 1.0);
}

pub(crate) fn init(app: &mut App) {
    app.insert_resource(Camera {
        eye: glam::vec3(0.0, 0.0, 1.0),
        dir: glam::vec3(0.0, 0.0, -1.0),
    });
    app.add_system(app::stages::UPDATE, camera_system.into_system());
}
