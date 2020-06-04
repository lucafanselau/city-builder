use nalgebra_glm as glm;

use winit::event::{DeviceEvent, Event, KeyboardInput, WindowEvent};

#[derive(Debug, Clone)]
enum MoveDirection {
    Forward,
    Backward,
    Left,
    Right,
    Up,
    Down,
    None,
}

#[derive(Debug)]
pub struct CameraData {
    view_projection: glm::Mat4,
}

#[derive(Debug)]
pub struct Camera {
    pub data: Option<glm::Mat4>,
    position: glm::Vec3,
    // view_dir: glm::Vec3,
    yaw: f32, // In degrees
    pitch: f32,
    sensitivity: f32,
    move_dir: MoveDirection,
    aspect_ratio: f32,
}

impl Camera {
    pub fn new(window: &winit::window::Window) -> Self {
        let inner_size = window.inner_size();
        Self {
            data: None,
            position: glm::vec3(-1., 0., 0.),
            // view_dir: glm::vec3(-1., -1., -1.),
            yaw: 0.0,
            pitch: 0.0,
            sensitivity: 0.3,
            move_dir: MoveDirection::None,
            aspect_ratio: inner_size.width as f32 / inner_size.height as f32,
        }
    }

    pub fn handle_event(&mut self, event: &Event<()>) {
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::KeyboardInput { input, .. } => {
                    self.move_dir = get_move_dir(&input).unwrap_or(self.move_dir.clone());
                }
                WindowEvent::Resized(dims) => {
                    self.aspect_ratio = dims.width as f32 / dims.height as f32
                }
                _ => (),
            },
            Event::DeviceEvent { event, .. } => match event {
                DeviceEvent::MouseMotion { delta } => {
                    // we want to move the camera according to this mouse movement
                    // info!("{:?}", delta);
                    self.yaw -= (delta.0 as f32) * self.sensitivity;
                    self.pitch -= (delta.1 as f32) * self.sensitivity;
                }
                _ => (),
            },
            _ => (),
        }
    }

    pub fn update(&mut self, dt: f32) {
        // self.position += self.dir * dt;
        // Calculate View Direction
        let view_dir = glm::vec3::<f32>(
            self.pitch.to_radians().cos() * self.yaw.to_radians().cos(),
            self.pitch.to_radians().sin(),
            self.pitch.to_radians().cos() * self.yaw.to_radians().sin(),
        );

        let up = glm::vec3(0., -1., 0.);
        let side_dir = glm::normalize(&glm::cross(&view_dir, &up));
        // for now constant
        let speed = 1.0f32;

        // Move if necessary
        match self.move_dir {
            MoveDirection::Forward => self.position += view_dir * dt * speed,
            MoveDirection::Backward => self.position -= view_dir * dt * speed,
            MoveDirection::Left => self.position -= side_dir * dt * speed,
            MoveDirection::Right => self.position += side_dir * dt * speed,
            MoveDirection::Up => self.position -= up * dt * speed,
            MoveDirection::Down => self.position += up * dt * speed,
            MoveDirection::None => (),
        }

        // info!("position: {:?}", self.position);

        let view_matrix = glm::look_at_rh(&self.position, &(self.position + view_dir), &up);

        let projection_matrix =
            glm::perspective_rh_zo(self.aspect_ratio, f32::to_radians(45.0), 0.1, 100.0);
        // projection_matrix.m22 *= -1.;

        self.data = Some(projection_matrix * view_matrix);
    }
}

fn get_move_dir(input: &KeyboardInput) -> Option<MoveDirection> {
    use winit::event::ElementState;
    if let Some(key_code) = input.virtual_keycode {
        use winit::event::VirtualKeyCode::*;
        if input.state == ElementState::Pressed {
            return match key_code {
                W => Some(MoveDirection::Forward),
                S => Some(MoveDirection::Backward),
                A => Some(MoveDirection::Left),
                D => Some(MoveDirection::Right),
                LShift => Some(MoveDirection::Down),
                RShift => Some(MoveDirection::Down),
                Space => Some(MoveDirection::Up),
                _ => None,
            };
        } else {
            // -> Released
            let key_codes = vec![W, A, S, D, LShift, RShift, Space];
            if key_codes.contains(&key_code) {
                return Some(MoveDirection::None);
            } else {
                return None;
            }
        }
    }
    None
}
