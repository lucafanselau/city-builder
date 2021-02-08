use glam::Vec2;
use std::cell::{Ref, RefMut};
use winit::event::{ElementState, VirtualKeyCode};

use app::{event::Events, stages, App, IntoFunctionSystem};

use crate::events::{CursorMoved, KeyboardInput};

#[derive(Debug)]
pub struct Input {
    pub mouse_pos: Vec2,
    pub mouse_delta: Vec2,
    keys: [ElementState; VirtualKeyCode::Cut as usize + 1],
}

impl Input {
    pub fn key(&self, key: VirtualKeyCode) -> ElementState {
        self.keys[key as usize]
    }

    pub fn is_pressed(&self, key: VirtualKeyCode) -> bool {
        self.key(key) == ElementState::Pressed
    }
}

fn input_system(
    mut input: RefMut<Input>,
    cursor_moved: Ref<Events<CursorMoved>>,
    keys: Ref<Events<KeyboardInput>>,
) {
    // Only use the last of the cursor events
    if let Some(CursorMoved(latest_cursor)) = cursor_moved.iter().last() {
        input.mouse_delta = input.mouse_pos - *latest_cursor;
        input.mouse_pos = *latest_cursor;
    } else {
        input.mouse_delta = glam::Vec2::zero()
    }

    for KeyboardInput { key, state } in keys.iter() {
        input.keys[*key as usize] = *state;
    }
}

pub(crate) fn init(app: &mut App) {
    app.insert_resource(Input {
        mouse_pos: glam::vec2(0.0, 0.0),
        mouse_delta: glam::vec2(0.0, 0.0),
        keys: [ElementState::Released; VirtualKeyCode::Cut as usize + 1],
    });

    app.add_system(stages::PREPARE_FRAME, input_system.into_system());
}
