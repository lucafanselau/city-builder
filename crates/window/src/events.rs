pub use winit::dpi::{PhysicalPosition, PhysicalSize};
pub use winit::event::{ElementState, VirtualKeyCode};

pub struct WindowResize(pub PhysicalSize<u32>);
pub struct CursorMoved {
    pub absolute: glam::Vec2,
    pub relative: glam::Vec2,
}
pub struct KeyboardInput {
    pub key: VirtualKeyCode,
    pub state: ElementState,
}
