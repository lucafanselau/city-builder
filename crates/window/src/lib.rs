extern crate winit;

use std::time::Instant;

use app::{event::Events, App};
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

pub mod events;

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

fn dispatch_event<T: app::event::Event>(r: &mut Resources, e: T) {
    let mut events = r
        .get_mut::<Events<T>>()
        .expect("[Window] (dispatch_event) event is not registered");
    events.send(e);
}

pub fn init_window(app: &mut App) {
    let event_loop = {
        let resources = app.get_resources();
        let size = winit::dpi::PhysicalSize::new(2400, 900);
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

    {
        // Register Events with the App
        app.add_event::<events::WindowResize>();
    }

    app.set_runner(|mut world, mut resources, mut scheduler| {
        let startup = Instant::now();

        event_loop.run(move |event, _, control_flow| {
            // ControlFlow::Poll continuously runs the event loop, even if the OS hasn't
            // dispatched any events. This is ideal for games and similar applications.
            *control_flow = ControlFlow::Poll;

            match event {
                Event::WindowEvent { event, .. } => {
                    // log::info!("{:#?}", event);
                    match event {
                        WindowEvent::CloseRequested => {
                            *control_flow = ControlFlow::Exit;
                            info!("The close button was pressed; stopping");
                        }
                        WindowEvent::Resized(size) => {
                            log::info!("Resized: {:?}", size);
                            {
                                // let mut window = resources
                                //     .get_mut::<WindowState>()
                                //     .expect("[window] failed to get window state");
                                // window.size = size;
                            }
                            dispatch_event(&mut resources, events::WindowResize(size));
                        }
                        _ => (),
                    }
                }
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
                    //
                    SequentialExecutor::execute(&mut scheduler, &mut world, &mut resources);
                }
                _ => (),
            }
        });
    })
}
