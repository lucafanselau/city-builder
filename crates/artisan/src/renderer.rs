use std::{path::Path, sync::Arc, time::Instant};

use app::{App, IntoMutatingSystem, Resources, World};
use bytemuck::{Pod, Zeroable};
use gfx::context::ContextBuilder as GfxContextBuilder;
use render::{
    context::GpuBuilder,
    graph::{node::Node, nodes::callbacks::FrameData, Graph},
    prelude::*,
    resource::{
        frame::Extent2D,
        pipeline::{
            AttributeDescriptor, GraphicsPipeline, PipelineShaders, PipelineState, PipelineStates,
            Primitive, Rasterizer, RenderContext as PipelineRenderContext, VertexAttributeFormat,
            VertexBufferDescriptor, VertexInputRate,
        },
        render_pass::{LoadOp, StoreOp},
    },
};

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

pub type ActiveContextBuilder = GfxContextBuilder;
pub type ActiveContext = <ActiveContextBuilder as GpuBuilder>::Context;

// struct RendererState {
//     vertex_buffer: Buffer<GfxContext>,
//     offset_buffer: SwapBuffer<GfxContext, Offset>,
//     glue_drops: Vec<Glue<GfxContext>>,
// }

pub fn init(app: &mut App) {
    let (ctx, surface) = {
        let resources = app.get_resources();
        let window_state = resources
            .get::<window::WindowState>()
            .expect("[Artisan] failed to load window");

        let mut ctx_builder = GfxContextBuilder::new();
        let window_size = window_state.window.inner_size();
        let surface = ctx_builder.create_surface(
            &window_state.window,
            Extent2D {
                width: window_size.width,
                height: window_size.height,
            },
        );
        let ctx = Arc::new(ctx_builder.build());
        (ctx, surface)
    };
    let resources = Arc::new(GpuResources::new(ctx.clone()));

    // Timer
    app.get_resources()
        .insert(Instant::now())
        .expect("[Artisan] failed to insert Instant resource");

    {
        let mut graph = ctx.create_graph(surface);

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
        let frames_in_flight = graph.get_swapchain_image_count();

        // TODO: Suboptimal
        // let vertex_buffer =
        //     resources.create_device_local_buffer("Vertex".into(), BufferUsage::Vertex, &vertices);
        let vertex_buffer = SwapBuffer::new(
            ctx.clone(),
            "Vertex Buffer".into(),
            frames_in_flight,
            vertices,
        );

        let initial_offset = Offset {
            // offset: [-0.2, 0.12, -0.4, 0.0],
            offset: [0.0, 0.0, 0.0, 0.0],
        };

        let offset_buffer = SwapBuffer::new(
            ctx.clone(),
            "Offset Uniform".into(),
            frames_in_flight,
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

        // TODO: load that from file or something

        let vertex_code = ctx.compile_shader(ShaderSource::GlslFile(
            Path::new("assets/shaders/simple.vert").into(),
        ));
        let fragment_code = ctx.compile_shader(ShaderSource::GlslFile(
            Path::new("assets/shaders/simple.frag").into(),
        ));

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

        let backbuffer = graph.get_backbuffer_attachment();
        {
            let mut builder =
                graph.build_pass_node::<GraphicsPipeline<ActiveContext>>("main_pass".into());
            builder.add_output(backbuffer, LoadOp::Clear, StoreOp::Store);
            builder.init(Box::new(move |rp| {
                // let _ctx = ctx.clone();
                let desc = GraphicsPipelineDescriptor {
                    name: "simple_pipeline".into(),
                    mixtures: vec![&mixture],
                    shaders: PipelineShaders {
                        vertex: vertex_code.clone(),
                        fragment: fragment_code.clone(),
                        geometry: None,
                    },
                    rasterizer: Rasterizer::FILL,
                    vertex_buffers: vec![VertexBufferDescriptor::new(
                        0,
                        vertex_size as _,
                        VertexInputRate::Vertex,
                    )],
                    attributes: vec![AttributeDescriptor::new(
                        0,
                        0,
                        0,
                        VertexAttributeFormat::Vec4,
                    )],
                    primitive: Primitive::TriangleList,
                    blend_targets: vec![true],
                    depth: None,
                    pipeline_states: PipelineStates {
                        viewport: PipelineState::Dynamic,
                        scissor: PipelineState::Dynamic,
                    },
                };

                let pipeline = resources
                    .create_graphics_pipeline(desc, PipelineRenderContext::RenderPass((rp, 0)));
                Box::new(pipeline)
            }));
            builder.callback(Box::new(move |frame, p, _w, resources| {
                let FrameData {
                    cmd,
                    frame_index,
                    viewport,
                } = frame;

                let elapsed = resources
                    .get::<Instant>()
                    .expect("[Artisan] failed to get timeing resource")
                    .elapsed()
                    .as_secs_f32();
                let offset = Offset {
                    offset: [elapsed.sin(), elapsed.cos(), elapsed.tan(), 0.0],
                };
                offset_buffer.write(offset);

                vertex_buffer.frame(frame_index);
                offset_buffer.frame(frame_index);

                cmd.bind_graphics_pipeline(&p);

                // TODO: Viewport
                cmd.set_viewport(0, viewport.clone());
                cmd.set_scissor(0, viewport.rect);

                cmd.bind_vertex_buffer(0, vertex_buffer.get(frame_index), BufferRange::WHOLE);

                cmd.snort_glue(0, &p, &glue_drops[frame_index as usize]);

                cmd.draw(0..3, 0..1);
            }));
            graph.add_node(Node::PassNode(builder.build()))
        }
        app.get_resources()
            .insert(graph)
            .expect("[Artisan] failed to insert Renderer State");
    };

    app.add_mut_system(frame_render.into_mut_system());
}

fn frame_render(world: &mut World, resources: &mut Resources) {
    let mut graph = resources
        .get_mut::<<ActiveContext as GpuContext>::ContextGraph>()
        .expect("[Artisan] failed to get graph");

    graph.execute(world, resources);

    // TODO: Execute graph

    // let ctx = &render_context.ctx;

    // let elapsed = timing.elapsed;
    // let offset = Offset {
    //     offset: [elapsed.sin(), elapsed.cos(), elapsed.tan(), 0.0],
    // };
    // state.offset_buffer.write(offset);

    // let (index, swapchain_image) = ctx.new_frame();

    // state.offset_buffer.frame(index);

    // let extent = window.size;
    // let framebuffer = ctx.create_framebuffer(
    //     &state.render_pass,
    //     vec![swapchain_image.borrow()],
    //     Extent3D {
    //         width: extent.width,
    //         height: extent.height,
    //         depth: 1,
    //     },
    // );

    // let viewport = Viewport {
    //     rect: Rect {
    //         x: 0,
    //         y: 0,
    //         width: extent.width as i16,
    //         height: extent.height as i16,
    //     },
    //     depth: 0.0..1.0,
    // };

    // let frame_commands = ctx.render_command(|cmd| {
    //     cmd.begin_render_pass(
    //         &state.render_pass,
    //         &framebuffer,
    //         viewport.clone().rect,
    //         vec![Clear::Color(0.34, 0.12, 0.12, 1.0)],
    //     );

    //     cmd.end_render_pass();
    // });

    // ctx.end_frame(swapchain_image, frame_commands);

    // ctx.drop_framebuffer(framebuffer);
}
