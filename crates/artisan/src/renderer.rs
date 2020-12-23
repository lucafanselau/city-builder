use std::{cell::Ref, path::Path};

use app::{App, IntoFunctionSystem};
use bytemuck::{Pod, Zeroable};
use gfx::gfx_context::Context as GfxContext;
use render::{
    prelude::*,
    resource::{
        frame::Clear,
        pipeline::{
            AttributeDescriptor, CullFace, Culling, GraphicsPipeline, PipelineShaders,
            PipelineState, PipelineStates, PolygonMode, Primitive, Rasterizer, Rect,
            RenderContext as PipelineRenderContext, VertexAttributeFormat, VertexBufferDescriptor,
            VertexInputRate, Viewport, Winding,
        },
        render_pass::{
            Attachment, AttachmentLoadOp, AttachmentStoreOp, RenderPassDescriptor,
            SubpassDescriptor,
        },
    },
    util::format::TextureLayout,
};
use window::{WindowState, WindowTiming};

use crate::RenderContext;

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

struct RendererState {
    vertex_buffer: Buffer<GfxContext>,
    offset_buffer: SwapBuffer<GfxContext, Offset>,
    render_pass: <GfxContext as GpuContext>::RenderPassHandle,
    pipeline: GraphicsPipeline<GfxContext>,
    glue_drops: Vec<Glue<GfxContext>>,
}

pub fn init(app: &mut App) {
    let state = {
        let render_context = app.get_resources().get::<RenderContext>().unwrap();
        let ctx = &render_context.ctx;
        let resources = &render_context.resources;

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

        let vertex_buffer = resources.create_device_local_buffer("Vertex".into(), &vertices);

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
        let parts = mixture![
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

            let pipeline = resources.create_graphics_pipeline(
                desc,
                PipelineRenderContext::RenderPass((&render_pass, 0)),
            );

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

        RendererState {
            vertex_buffer,
            offset_buffer,
            render_pass,
            pipeline,
            glue_drops,
        }
    };

    app.get_resources()
        .insert(state)
        .expect("[Artisan] failed to insert Renderer State");

    app.add_system(app::stages::RENDER, frame_render.into_system());
}

fn frame_render(
    render_context: Ref<RenderContext>,
    window: Ref<WindowState>,
    state: Ref<RendererState>,
    timing: Ref<WindowTiming>,
) {
    use std::borrow::Borrow;
    use std::ops::Deref;

    let ctx = &render_context.ctx;

    let elapsed = timing.elapsed;
    let offset = Offset {
        offset: [elapsed.sin(), elapsed.cos(), elapsed.tan(), 0.0],
    };
    state.offset_buffer.write(offset);

    let (index, swapchain_image) = ctx.new_frame();

    state.offset_buffer.frame(index);

    let extent = window.size;
    let framebuffer = ctx.create_framebuffer(
        &state.render_pass,
        vec![swapchain_image.borrow()],
        Extent3D {
            width: extent.width,
            height: extent.height,
            depth: 1,
        },
    );

    let viewport = Viewport {
        rect: Rect {
            x: 0,
            y: 0,
            width: extent.width as i16,
            height: extent.height as i16,
        },
        depth: 0.0..1.0,
    };

    let frame_commands = ctx.render_command(|cmd| {
        cmd.begin_render_pass(
            &state.render_pass,
            &framebuffer,
            viewport.clone().rect,
            vec![Clear::Color(0.34, 0.12, 0.12, 1.0)],
        );

        cmd.bind_graphics_pipeline(&state.pipeline);

        cmd.set_viewport(0, viewport.clone());
        cmd.set_scissor(0, viewport.rect);

        cmd.bind_vertex_buffer(0, state.vertex_buffer.deref(), BufferRange::WHOLE);

        cmd.snort_glue(0, &state.pipeline, &state.glue_drops[index as usize]);

        cmd.draw(0..3, 0..1);

        cmd.end_render_pass();
    });

    ctx.end_frame(swapchain_image, frame_commands);

    ctx.drop_framebuffer(framebuffer);
}
