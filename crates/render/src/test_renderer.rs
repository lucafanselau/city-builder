use crate::command_encoder::CommandEncoder;
use crate::context::{create_render_context, GpuContext};
use crate::resource::buffer::{BufferDescriptor, BufferUsage, MemoryType};
use crate::resource::frame::{Clear, Extent3D};
use crate::resource::pipeline::{
    CullFace, Culling, GraphicsPipelineDescriptor, PipelineShaders, PipelineState, PipelineStates,
    PolygonMode, Primitive, Rasterizer, Rect, RenderContext, ShaderSource, Viewport, Winding,
};
use crate::resource::render_pass::{
    Attachment, AttachmentLoadOp, AttachmentStoreOp, RenderPassDescriptor, SubpassDescriptor,
};
use crate::resources::GpuResources;
use crate::util::format::TextureLayout;
use bytemuck::{Pod, Zeroable};
use gfx_hal::image::Extent;
use log::info;
use raw_window_handle::HasRawWindowHandle;
use std::borrow::Borrow;
use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

#[derive(Copy, Clone, Zeroable, Pod)]
#[repr(C)]
struct SampleData {
    a: u32,
    b: u32,
}

pub fn test_renderer<W: HasRawWindowHandle>(w: &W, extent: (u32, u32)) {
    let ctx = Arc::new(create_render_context(w));

    info!("Surface format is {:#?}", ctx.get_surface_format());

    let resources = GpuResources::new(ctx.clone());

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

    let (pipeline, render_pass) = {
        let vertex_code = ctx.compile_shader(ShaderSource::GlslFile(
            Path::new("assets/shaders/simple.vert").into(),
        ));
        let fragment_code = ctx.compile_shader(ShaderSource::GlslFile(
            Path::new("assets/shaders/simple.frag").into(),
        ));

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
            pipeline_states: PipelineStates {
                viewport: PipelineState::Dynamic,
                scissor: PipelineState::Dynamic,
            },
        };

        let pipeline =
            resources.create_graphics_pipeline(&desc, RenderContext::RenderPass((&render_pass, 0)));

        (pipeline, render_pass)
    };

    let mut running = true;
    let now = std::time::Instant::now();
    while running {
        if now.elapsed().as_millis() > 2000 {
            running = false;
        }

        let swapchain_image = ctx.new_frame();

        let framebuffer = ctx.create_framebuffer(
            &render_pass,
            vec![swapchain_image.borrow()],
            Extent3D {
                width: extent.0,
                height: extent.1,
                depth: 1,
            },
        );

        let viewport = Viewport {
            rect: Rect {
                x: 0,
                y: 0,
                width: extent.0 as i16,
                height: extent.1 as i16,
            },
            depth: 0.0..1.0,
        };

        let frame_commands = ctx.render_command(|cmd| {
            cmd.begin_render_pass(
                &render_pass,
                &framebuffer,
                viewport.clone().rect,
                vec![Clear::Color(0.34, 0.12, 0.12, 1.0)],
            );

            cmd.bind_graphics_pipeline(&pipeline);

            // TODO: Dynamic viewport and scissor
            cmd.set_viewport(0, viewport.clone());
            cmd.set_scissor(0, viewport.rect);

            cmd.draw(0..3, 0..1);

            cmd.end_render_pass();
        });

        ctx.end_frame(swapchain_image, frame_commands);

        ctx.drop_framebuffer(framebuffer);
    }

    ctx.wait_idle();

    ctx.drop_render_pass(render_pass);
}
