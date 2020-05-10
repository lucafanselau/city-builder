mod renderer;
mod window;

use winit::event::{Event, WindowEvent};
use winit::event_loop::ControlFlow;

use log::*;
use simplelog;

use gfx_backend_vulkan as back;

fn main() {
    let _logger = {
				use simplelog::{ ConfigBuilder, TermLogger, TerminalMode };
				
        let config = ConfigBuilder::new()
            .set_location_level(LevelFilter::Warn)
						.build();

        TermLogger::init(
            LevelFilter::max(),
            config,
            TerminalMode::Mixed,
        )
        .unwrap()
    };

    let window_size = winit::dpi::LogicalSize::new(1600, 900);

    let (event_loop, window) =
        window::create_window("Mightycity", window_size).expect("failed to create a window");

    let mut r = match renderer::Renderer::<back::Backend>::new(&window) {
        Ok(renderer) => renderer,
        Err(e) => {
            error!("Renderer: Creation Failed");
            error!("{}", e);
            panic!(e.to_string());
        }
    };

		let start_time = std::time::Instant::now();

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
                WindowEvent::Resized(dims) => r.handle_resize(dims),

                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    r.handle_resize(*new_inner_size)
                }
                _ => (),
            },
            Event::MainEventsCleared => {
                // Updating
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                // Here happens rendering
                match r.render(&start_time) {
                    Ok(_) => (),
                    Err(e) => {
                        error!("Renderer: Rendering Failed");
                        error!("{}", e);
                        panic!(e.to_string());
                    }
                }
            }
            _ => (),
        }
    });
}
