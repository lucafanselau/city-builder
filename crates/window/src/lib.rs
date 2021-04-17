use app::{App, Events};
use ecs::{prelude::*, schedule::executor::ScheduleExecutor};

use log::info;
use winit::{
    dpi::{PhysicalSize, Size},
    event_loop::ControlFlow,
};
use winit::{
    error::OsError,
    event::{Event, WindowEvent},
};
use winit::{event::KeyboardInput, event_loop::EventLoop};
use winit::{
    event::{ElementState, VirtualKeyCode},
    window::{Window, WindowBuilder},
};

pub mod events;
pub mod input;

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

fn dispatch_event<T: app::Event>(r: &mut Resources, e: T) {
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

        event_loop
    };

    {
        // Register Events with the App
        app.add_event::<events::WindowResize>();
        app.add_event::<events::CursorMoved>();
        app.add_event::<events::KeyboardInput>();

        // Initialize Submodules
        input::init(app);
    }

    app.set_runner(|mut resources, mut world, mut scheduler| {
        event_loop.run(move |event, _, control_flow| {
            // ControlFlow::Poll continuously runs the event loop, even if the OS hasn't
            // dispatched any events. This is ideal for games and similar applications.
            *control_flow = ControlFlow::Poll;

            match event {
                Event::WindowEvent { event, .. } => {
                    // log::info!("{:#?}", event);
                    match event {
                        WindowEvent::CursorMoved { position, .. } => {
                            let absolute = glam::vec2(position.x as _, position.y as _);
                            let relative = {
                                let window = resources.get::<WindowState>().unwrap();
                                glam::vec2(
                                    absolute.x / window.size.width as f32,
                                    absolute.y / window.size.height as f32,
                                )
                            };
                            dispatch_event(
                                &mut resources,
                                events::CursorMoved { absolute, relative },
                            );
                        }
                        WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    virtual_keycode: Some(key),
                                    state,
                                    ..
                                },
                            ..
                        } => {
                            if key == VirtualKeyCode::Escape && state == ElementState::Pressed {
                                log::info!("Escape was pressed; stopping");
                                *control_flow = ControlFlow::Exit;
                            }
                            dispatch_event(&mut resources, events::KeyboardInput { key, state })
                        }

                        WindowEvent::CloseRequested => {
                            *control_flow = ControlFlow::Exit;
                            info!("The close button was pressed; stopping");
                        }
                        WindowEvent::Resized(size) => {
                            log::info!("Resized: {:?}", size);
                            dispatch_event(&mut resources, events::WindowResize(size));
                            if let Ok(mut window) = resources.get_mut::<WindowState>() {
                                window.size = size;
                            }
                        }
                        _ => (),
                    }
                }
                Event::MainEventsCleared => {
                    let window_state = resources.get::<WindowState>().unwrap();
                    window_state.window.request_redraw();
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
