extern crate winit;

use std::time::Instant;

use app::App;
use ecs::{prelude::*, schedule::executor::ScheduleExecutor};

use log::info;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};
use winit::{
    dpi::{PhysicalSize, Size},
    event_loop::ControlFlow,
};
use winit::{
    error::OsError,
    event::{Event, WindowEvent},
};

fn create_window<T: Into<String>, S: Into<Size>>(
    title: T,
    size: S,
) -> Result<(EventLoop<()>, Window), OsError> {
    let event_loop = EventLoop::new();
    let builder_result = WindowBuilder::new()
        .with_title(title)
        .with_inner_size(size)
        .build(&event_loop);

    builder_result.map(|window| (event_loop, window))
}

#[derive(Debug)]
pub struct WindowState {
    pub window: Window,
    // pub event_loop: EventLoop<()>,
    pub size: PhysicalSize<u32>,
}

#[derive(Debug)]
pub struct WindowTiming {
    pub elapsed: f32,
    pub delta_time: f32,
}

pub fn init_window(app: &mut App) {
    let event_loop = {
        let resources = app.get_resources();
        let size = winit::dpi::LogicalSize::new(1600, 900);
        let (event_loop, window) =
            create_window("City Builder", size).expect("[Window] failed to build window");

        let size = window.inner_size();

        let state = WindowState {
            window,
            // event_loop,
            size,
        };

        resources
            .insert(state)
            .expect("[Window] failed to insert window state");

        resources
            .insert(WindowTiming {
                elapsed: 0f32,
                delta_time: 0f32,
            })
            .expect("[Window] failed to insert initial window timing");

        event_loop
    };

    app.set_runner(|world, resources, scheduler| {
        let startup = Instant::now();

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
                    {
                        let elapsed = startup.elapsed().as_secs_f32();
                        let mut timing = resources.get_mut::<WindowTiming>().unwrap();
                        let delta_time = elapsed - timing.elapsed;
                        timing.delta_time = delta_time;
                        timing.elapsed = elapsed;
                    }
                    {
                        let window_state = resources.get::<WindowState>().unwrap();
                        window_state.window.request_redraw();
                    }
                }
                Event::RedrawRequested(_) => {
                    // This dumb af
                    SequentialExecutor::execute(&scheduler, &world, &resources);
                }
                _ => (),
            }
        });
    })
}
