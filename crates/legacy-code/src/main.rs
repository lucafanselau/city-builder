fn old_main() {
    let _logger = {
        use simplelog::{ConfigBuilder, TermLogger, TerminalMode};

        let config = ConfigBuilder::new()
            .set_location_level(LevelFilter::Warn)
            .build();

        TermLogger::init(LevelFilter::max(), config, TerminalMode::Mixed).unwrap()
    };

    let window_size = winit::dpi::LogicalSize::new(1600, 900);

    let (event_loop, window) =
        window::create_window("Mightycity", window_size).expect("failed to create a window");

    let mut r = match renderer::Renderer::<back::Backend>::new(window.clone()) {
        Ok(renderer) => renderer,
        Err(e) => {
            error!("Renderer: Creation Failed");
            error!("{}", e);
            panic!(e.to_string());
        }
    };

    let start_time = std::time::Instant::now();
    let mut previous_time = start_time.clone();

    let mut camera = Camera::new(&window);

    window
        .set_cursor_grab(true)
        .expect("failed to set cursor grap");

    let mut ui: Option<UiHandle> = None;

    event_loop.run(move |event, _, control_flow| {
        // ControlFlow::Poll continuously runs the event loop, even if the OS hasn't
        // dispatched any events. This is ideal for games and similar applications.
        *control_flow = ControlFlow::Poll;

        {
            camera.handle_event(&event);
        }
        {
            r.handle_event(&event);
        }
        match event {
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                        info!("The close button was pressed; stopping");
                    }
                    WindowEvent::Resized(dims) => r.handle_resize(dims),
                    WindowEvent::KeyboardInput { input, .. } => {
                        if input.state == ElementState::Pressed {
                            if let Some(key_code) = input.virtual_keycode {
                                if key_code == VirtualKeyCode::Escape {
                                    // exit application
                                    *control_flow = ControlFlow::Exit;
                                    info!("Escape was pressed, closing now!");
                                }
                            }
                        }
                    }
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        r.handle_resize(*new_inner_size)
                    }
                    _ => (),
                }
            }
            Event::MainEventsCleared => {
                // Updating
                // Calculate Delta Time
                let now = std::time::Instant::now();
                let dt = now.duration_since(previous_time).as_secs_f32();
                previous_time = now;

                ui = Some(r.update());

                if let Some(ui_ref) = ui.as_ref() {
                    let mut camera_pos = camera.position;

                    let ui_handle = &ui_ref.ui;
                    imgui::Window::new(im_str!("Game")).build(ui_handle, || {
                        ui_handle.text(im_str!("Auch von hier HIII!"));
                        Slider::new(im_str!("Pos X"), -10.0..=10.0)
                            .build_array(ui_handle, camera_pos.as_mut_slice());
                    });

                    camera.position = camera_pos;
                }

                camera.update(dt);

                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                // Here happens rendering
                match r.render(&start_time, &camera, ui.take().expect("failed to get ui")) {
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
