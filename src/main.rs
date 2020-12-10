mod camera;
mod graphics;
mod window;

use graphics::renderer;
use winit::event::{ElementState, Event, VirtualKeyCode, WindowEvent};
use winit::event_loop::ControlFlow;

use log::*;
use simplelog;

use camera::Camera;

use imgui::{im_str, Slider};

use crate::graphics::renderer::ui::UiHandle;
use gfx_backend_vulkan as back;

use ecs::prelude::*;
use render;
use render::resource::buffer::{BufferDescriptor, BufferUsage, MemoryType};
use std::borrow::Borrow;
use std::sync::Arc;

use bytemuck::__core::time::Duration;
use bytemuck::{Pod, Zeroable};
use render::context::GpuContext;
use render::resource::pipeline::{
    CullFace, Culling, GraphicsPipelineDescriptor, PipelineShaders, PipelineState, PolygonMode,
    Primitive, Rasterizer, RenderContext, ShaderType, Viewport, Winding,
};
use render::resource::render_pass::{
    Attachment, AttachmentLoadOp, AttachmentStoreOp, RenderPassDescriptor, SubpassDescriptor,
};
use render::util::format::TextureLayout;
use std::ops::Deref;

#[derive(Copy, Clone, Zeroable, Pod)]
#[repr(C)]
struct SampleData {
    a: u32,
    b: u32,
}

const VERTEX_SHADER: &str = r#"
#version 450

vec2 positions[3] = vec2[](
    vec2(0.0, -0.5),
    vec2(0.5, 0.5),
    vec2(-0.5, 0.5)
);

void main() {
    gl_Position = vec4(positions[gl_VertexIndex], 0.0, 1.0);
}
"#;

const FRAGMENT_SHADER: &str = r#"
#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) out vec4 outColor;

void main() {
    outColor = vec4(1.0, 0.0, 0.0, 1.0);
}"#;

fn main() {
    let _logger = {
        use simplelog::{ConfigBuilder, TermLogger, TerminalMode};

        let config = ConfigBuilder::new()
            .set_location_level(LevelFilter::Warn)
            .build();

        TermLogger::init(LevelFilter::Warn, config, TerminalMode::Mixed).unwrap()
    };

    let _schedule = {
        let s = Scheduler::new();
        s
    };

    let window_size = winit::dpi::LogicalSize::new(1600, 900);

    let (_event_loop, window) =
        window::create_window("Mightycity", window_size).expect("failed to create a window");

    let ctx = Arc::new(render::context::create_render_context::<
        winit::window::Window,
    >(window.borrow()));

    info!("Surface format is {:#?}", ctx.get_surface_format());

    let resources = render::resources::GpuResources::new(ctx.clone());

    let buffer = resources.create_empty_buffer(BufferDescriptor {
        name: "test_buffer".into(),
        size: 4,
        memory_type: MemoryType::HostVisible,
        usage: BufferUsage::Uniform,
    });

    let sample_data = SampleData { a: 17, b: 21 };
    unsafe {
        ctx.write_to_buffer(buffer.deref(), &sample_data);
    }

    let _pipeline = {
        use render::resource::pipeline::ShaderSource;
        let vertex_code = ctx.compile_shader(ShaderSource::GlslSource((
            VERTEX_SHADER,
            ShaderType::Vertex,
            Some("simple_vertex_shader"),
        )));
        let fragment_code = ctx.compile_shader(ShaderSource::GlslSource((
            FRAGMENT_SHADER,
            ShaderType::Fragment,
            Some("simple_fragment_shader"),
        )));

        let render_pass = {
            let color_attachment = Attachment {
                format: ctx.get_surface_format(),
                load_op: AttachmentLoadOp::Clear,
                store_op: AttachmentStoreOp::Store,
                layouts: TextureLayout::Undefined..TextureLayout::Present,
            };

            let subpass = SubpassDescriptor {
                colors: vec![(0, TextureLayout::ColorAttachmentOptimal)],
                depth_stencil: None,
                inputs: vec![],
                resolves: vec![],
                preserves: vec![],
            };

            let desc = RenderPassDescriptor {
                attachments: vec![color_attachment],
                subpasses: vec![subpass],
                pass_dependencies: vec![],
            };

            ctx.create_render_pass(&desc)
        };

        let desc = GraphicsPipelineDescriptor {
            name: "simple_pipeline".into(),
            shaders: PipelineShaders {
                vertex: vertex_code,
                fragment: fragment_code,
                geometry: None,
            },
            rasterizer: Rasterizer {
                polygon_mode: PolygonMode::Fill,
                culling: Culling {
                    winding: Winding::Clockwise,
                    cull_face: CullFace::None,
                },
            },
            vertex_buffers: vec![],
            attributes: vec![],
            primitive: Primitive::TriangleList,
            blend_targets: vec![true],
            depth: None,
            viewport: Viewport {
                viewport: PipelineState::Dynamic,
                scissor: PipelineState::Dynamic,
            },
        };

        let pipeline =
            resources.create_graphics_pipeline(&desc, RenderContext::RenderPass((&render_pass, 0)));

        ctx.drop_render_pass(render_pass);

        pipeline
    };

    std::thread::sleep(Duration::from_secs(2));
}

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
