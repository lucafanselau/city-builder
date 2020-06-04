extern crate winit;

use std::rc::Rc;

use winit::dpi::LogicalSize;
use winit::error::OsError;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

pub fn create_window<T: Into<String>>(
    title: T,
    size: LogicalSize<u32>,
) -> Result<(EventLoop<()>, Rc<Window>), OsError> {
    let event_loop = EventLoop::new();
    let builder_result = WindowBuilder::new()
        .with_title(title)
        .with_inner_size(size)
        .build(&event_loop);

    builder_result.map(|window| (event_loop, Rc::new(window)))
}
