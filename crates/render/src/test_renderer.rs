use crate::resource::frame::{Clear, Extent3D};
use crate::resource::pipeline::{
    AttributeDescriptor, CullFace, Culling, GraphicsPipelineDescriptor, PipelineShaders,
    PipelineState, PipelineStates, PolygonMode, Primitive, Rasterizer, Rect, RenderContext,
    ShaderSource, VertexAttributeFormat, VertexBufferDescriptor, VertexInputRate, Viewport,
    Winding,
};
use crate::resource::render_pass::{
    Attachment, AttachmentLoadOp, AttachmentStoreOp, RenderPassDescriptor, SubpassDescriptor,
};
use crate::resource::{
    buffer::{BufferDescriptor, BufferRange, BufferUsage, MemoryType},
    swap_buffer::SwapBuffer,
};
use crate::resources::GpuResources;
use crate::util::format::TextureLayout;
use crate::{command_encoder::CommandEncoder, resource::glue};
use crate::{
    context::{create_render_context, GpuContext},
    resource::glue::PartIndex,
};
use bytemuck::{Pod, Zeroable};
use log::info;
use log::warn;
use raw_window_handle::HasRawWindowHandle;
use std::borrow::Borrow;
use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

#[derive(Copy, Clone, Zeroable, Pod)]
#[repr(C)]
struct Vertex {
    pos: [f32; 4],
}

#[derive(Copy, Clone, Zeroable, Pod)]
#[repr(C)]
struct Offset {
    offset: [f32; 4],
}

#[derive(Copy, Clone)]
#[repr(C)]
struct ShaderCamera {
    view_projection: [[f32; 4]; 4],
}

pub fn test_renderer<W: HasRawWindowHandle>(w: &W, extent: (u32, u32)) {
    let ctx = Arc::new(create_render_context(w));

    info!("Surface format is {:#?}", ctx.get_surface_format());

    let resources = GpuResources::new(ctx.clone());

    let vertices = [
        Vertex {
            pos: [-0.5, -0.5, 0.0, 1.0],
        },
        Vertex {
            pos: [0.0, 0.5, 0.0, 1.0],
        },
        Vertex {
            pos: [0.5, -0.5, 0.0, 1.0],
        },
    ];
    let vertex_size = std::mem::size_of::<Vertex>();

    let vertex_buffer = resources.create_vertex_buffer("Vertex".into(), &vertices);

    let frames_in_flight = ctx.swapchain_image_count();

    let initial_offset = Offset {
        offset: [-0.2, 0.12, -0.4, 0.0],
    };

    let offset_buffer = SwapBuffer::new(
        ctx.clone(),
        "Offset Uniform".into(),
        ctx.swapchain_image_count(),
        initial_offset,
    );

    // Which is the equivalent of a DescriptorSetLayout
    let parts = crate::mixture![
        0: "offset" in Vertex: Offset
        // 1: "camera" in Vertex: ShaderCamera,
        // 2: "material" in Fragment: [dynamic Material],
        // 2: "albedo" in Fragment: sampler
    ];

    let mixture = resources.stir(parts);

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
            mixtures: vec![&mixture],
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
            vertex_buffers: vec![VertexBufferDescriptor {
                binding: 0,
                stride: vertex_size as u32,
                rate: VertexInputRate::Vertex,
            }],
            attributes: vec![AttributeDescriptor {
                location: 0,
                binding: 0,
                offset: 0,
                format: VertexAttributeFormat::Vec4,
            }],
            primitive: Primitive::TriangleList,
            blend_targets: vec![true],
            depth: None,
            pipeline_states: PipelineStates {
                viewport: PipelineState::Dynamic,
                scissor: PipelineState::Dynamic,
            },
        };

        let pipeline =
            resources.create_graphics_pipeline(desc, RenderContext::RenderPass((&render_pass, 0)));

        (pipeline, render_pass)
    };

    let mut glue_drops = Vec::with_capacity(frames_in_flight);

    for i in 0..frames_in_flight {
        let mut glue_bottle = resources.bottle(&mixture);
        glue_bottle.write_buffer(
            PartIndex::Name("offset".into()),
            offset_buffer.get(i as u32),
            None,
        );
        glue_drops.push(glue_bottle.apply());
    }

    let mut running = true;
    let startup = Instant::now();
    let mut last_frame = startup.clone();
    let mut counter = 0f64;
    let mut frames = 0u32;
    warn!("starting..");
    while running {
        let elapsed = startup.elapsed().as_secs_f32();
        if elapsed > 6.0 {
            running = false;
        }

        let offset = Offset {
            offset: [elapsed.sin(), elapsed.cos(), elapsed.tan(), 0.0],
        };

        offset_buffer.write(offset);

        let now = Instant::now();

        {
            frames += 1;
            let delta = now.duration_since(last_frame).as_secs_f64();
            // warn!("frame: delta: {}", delta);
            last_frame = now.clone();
            counter += delta;
            if counter >= 1f64 {
                counter -= 1f64;
                info!("fps: {}", frames);
                frames = 0;
            }
        }

        let (index, swapchain_image) = ctx.new_frame();

        offset_buffer.frame(index);

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

            cmd.set_viewport(0, viewport.clone());
            cmd.set_scissor(0, viewport.rect);

            cmd.bind_vertex_buffer(0, vertex_buffer.deref(), BufferRange::WHOLE);

            cmd.snort_glue(0, &pipeline, &glue_drops[index as usize]);

            cmd.draw(0..3, 0..1);

            cmd.end_render_pass();
        });

        ctx.end_frame(swapchain_image, frame_commands);

        ctx.drop_framebuffer(framebuffer);
    }

    ctx.wait_idle();

    ctx.drop_render_pass(render_pass);
}
