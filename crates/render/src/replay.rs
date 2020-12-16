use crate::context::{create_render_context, CurrentContext, GpuContext};
use crate::resources::GpuResources;
use log::info;
use std::io::stdin;
use std::sync::Arc;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};

fn init_simplelog() {
    use simplelog::*;
    use simplelog::{ConfigBuilder, TermLogger, TerminalMode};

    let config = ConfigBuilder::new()
        .add_filter_ignore_str("gfx_backend_vulkan")
        .set_ignore_level(LevelFilter::Info)
        .build();

    TermLogger::init(LevelFilter::Info, config, TerminalMode::Mixed).unwrap()
}

pub fn create_context() -> (Arc<CurrentContext>, GpuResources, Window, EventLoop<()>) {
    init_simplelog();

    let old_handler = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        println!("uh oh!");
        old_handler(info);
        let mut line = String::new();
        stdin().read_line(&mut line).unwrap();
    }));

    let window_size = winit::dpi::LogicalSize::new(1600, 900);

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("test suite")
        .with_inner_size(window_size)
        .build(&event_loop)
        .expect("[replay] failed to build window");

    let ctx = Arc::new(create_render_context(&window));

    info!("Surface format is {:#?}", ctx.get_surface_format());
    let resources = GpuResources::new(ctx.clone());
    (ctx, resources, window, event_loop)
}

pub fn run_loop(window: Window, event_loop: EventLoop<()>, mut cb: impl FnMut() + 'static) {
    event_loop.run(move |event, _, control_flow| {
        // ControlFlow::Poll continuously runs the event loop, even if the OS hasn't
        // dispatched any events. This is ideal for games and similar applications.
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                    info!("The close button was pressed; stopping");
                }
                _ => (),
            },
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                cb();
            }
            _ => (),
        }
    });
}
