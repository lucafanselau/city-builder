use std::{any::TypeId, path::Path, sync::Arc, time::Instant};

use app::{App, IntoMutatingSystem, Resources, World};
use bytemuck::{Pod, Zeroable};
use gfx::context::ContextBuilder as GfxContextBuilder;
use glam::Vec3A;
use render::{
    context::GpuBuilder,
    graph::{
        attachment::{AttachmentSize, GraphAttachment},
        builder::GraphBuilder,
        node::Node,
        nodes::callbacks::FrameData,
        Graph,
    },
    prelude::*,
    resource::{
        frame::Extent2D,
        pipeline::{
            GraphicsPipeline, PipelineShaders, PipelineState, PipelineStates, Primitive,
            Rasterizer, RenderContext as PipelineRenderContext,
        },
        render_pass::{LoadOp, StoreOp},
    },
};

use crate::{
    camera::{Camera, CameraBuffer},
    components::MeshComponent,
    material::{MaterialComponent, SolidMaterial},
    mesh::{MeshMap, Vertex},
};

const LIGHT_POSITION: Vec3A = glam::const_vec3a!([10.0, 10.0, 10.0]);

#[derive(Copy, Clone)]
#[repr(C)]
struct Light {
    light_position: Vec3A,
    view_position: Vec3A,
}

unsafe impl Zeroable for Light {}
unsafe impl Pod for Light {}

pub type ActiveContextBuilder = GfxContextBuilder;
pub type ActiveContext = <ActiveContextBuilder as GpuBuilder>::Context;

// struct RendererState {
//     vertex_buffer: Buffer<GfxContext>,
//     offset_buffer: SwapBuffer<GfxContext, Offset>,
//     glue_drops: Vec<Glue<GfxContext>>,
// }

pub fn init(app: &mut App) {
    let (ctx, surface, initial_aspect_ratio) = {
        let resources = app.get_resources();
        let window_state = resources
            .get::<window::WindowState>()
            .expect("[Artisan] failed to load window");

        let mut ctx_builder = GfxContextBuilder::new();
        let window_size = window_state.window.inner_size();
        let initial_aspect_ratio = window_size.width as f32 / window_size.height as f32;
        let surface = ctx_builder.create_surface(
            &window_state.window,
            Extent2D {
                width: window_size.width,
                height: window_size.height,
            },
        );
        let ctx = Arc::new(ctx_builder.build());
        (ctx, surface, initial_aspect_ratio)
    };
    let resources = Arc::new(GpuResources::new(ctx.clone()));

    {
        // Insert MeshMap
        app.get_resources()
            .insert(MeshMap::new(ctx.clone()))
            .expect("[Artisan] failed to insert mesh map");
    }

    {
        let mut graph_builder = ctx.create_graph(surface);
        let frames_in_flight = graph_builder.get_swapchain_image_count();

        let initial_camera = {
            let camera = app.get_res::<Camera>();
            camera.calc(initial_aspect_ratio)
        };

        let camera_buffer = SwapBuffer::new(
            ctx.clone(),
            "Camera Uniform".into(),
            frames_in_flight,
            initial_camera,
        );

        let initial_light = {
            let camera = app.get_res::<Camera>();
            Light {
                light_position: LIGHT_POSITION,
                view_position: camera.eye.into(),
            }
        };

        let light_buffer = SwapBuffer::new(
            ctx.clone(),
            "Light Uniform".into(),
            frames_in_flight,
            initial_light,
        );

        // Which is the equivalent of a DescriptorSetLayout
        let parts = mixture![
            0: "camera_buffer" in Vertex: CameraBuffer,
            1: "light" in Fragment: Light
            // 1: "camera" in Vertex: ShaderCamera,
            // 2: "material" in Fragment: [dynamic Material],
            // 2: "albedo" in Fragment: sampler
        ];

        let mixture = resources.stir(parts);

        // TODO: load that from file or something

        let vertex_code = ctx.compile_shader(ShaderSource::GlslFile(
            Path::new("assets/shaders/solid.vert").into(),
        ));
        let fragment_code = ctx.compile_shader(ShaderSource::GlslFile(
            Path::new("assets/shaders/solid.frag").into(),
        ));

        let mut glue_drops = Vec::with_capacity(frames_in_flight);

        for i in 0..frames_in_flight {
            let mut glue_bottle = resources.bottle(&mixture);
            glue_bottle.write_buffer(
                PartIndex::Name("camera_buffer".into()),
                camera_buffer.get(i as u32),
                None,
            );
            glue_bottle.write_buffer(PartIndex::Binding(1), light_buffer.get(i as _), None);
            glue_drops.push(glue_bottle.apply());
        }

        let backbuffer = graph_builder.get_backbuffer_attachment();
        // Depth attachment
        let depth_attachment = graph_builder.add_attachment(GraphAttachment::new(
            "Depth Attachment",
            AttachmentSize::SWAPCHAIN,
            graph_builder.default_depth_format(),
        ));

        {
            let mut builder = graph_builder
                .build_pass_node::<GraphicsPipeline<ActiveContext>>("main_pass".into());
            builder.add_output(backbuffer, LoadOp::Clear, StoreOp::Store);
            // builder.set_depth()
            builder.init(Box::new(move |rp| {
                // let _ctx = ctx.clone();
                let (buffer_descriptor, attributes) = Vertex::get_layout();
                let desc = GraphicsPipelineDescriptor {
                    name: "simple_pipeline".into(),
                    mixtures: vec![&mixture],
                    push_constants: vec![(
                        ShaderType::Fragment,
                        0..(std::mem::size_of::<SolidMaterial>() as _),
                    )],
                    shaders: PipelineShaders {
                        vertex: vertex_code.clone(),
                        fragment: fragment_code.clone(),
                        geometry: None,
                    },
                    rasterizer: Rasterizer::FILL,
                    vertex_buffers: vec![buffer_descriptor],
                    attributes,
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
            builder.callback(Box::new(move |frame, p, world, resources| {
                let FrameData {
                    cmd,
                    frame_index,
                    viewport,
                } = frame;

                {
                    // Calculate new camera
                    let camera = resources.get::<Camera>().unwrap();
                    let camera_data = {
                        let aspect_ratio = viewport.rect.width as f32 / viewport.rect.height as f32;
                        camera.calc(aspect_ratio)
                    };
                    camera_buffer.write(camera_data);
                    camera_buffer.frame(frame_index);
                    // Update Light Buffer
                    let light_data = Light {
                        light_position: LIGHT_POSITION,
                        view_position: camera.eye.into(),
                    };
                    light_buffer.write(light_data);
                    light_buffer.frame(frame_index);
                }

                let mesh_map = resources
                    .get::<MeshMap>()
                    .expect("[Artisan] (renderer) failed to get mesh map");

                cmd.bind_graphics_pipeline(&p);

                // TODO: Viewport
                cmd.set_viewport(0, viewport.clone());
                cmd.set_scissor(0, viewport.rect);

                cmd.snort_glue(0, &p, &glue_drops[frame_index as usize]);

                for (_e, (mesh, mat)) in
                    world.query::<(&MeshComponent, &MaterialComponent)>().iter()
                {
                    if let MaterialComponent::Solid(ref solid) = mat {
                        let (vertex_count, buffer) = mesh_map.draw_info(&mesh.0);

                        let push_data: &[u32] = bytemuck::cast_slice(bytemuck::bytes_of(solid));
                        // log::info!("Push Data is: \n{:#?}", push_data);
                        cmd.push_constants(&p, ShaderType::Fragment, 0, push_data);

                        cmd.bind_vertex_buffer(0, buffer, BufferRange::WHOLE);
                        cmd.draw(0..vertex_count, 0..1);
                    } else {
                        unimplemented!();
                    }
                }
            }));
            graph_builder.add_node(Node::PassNode(builder.build()))
        }
        app.get_resources()
            .insert(graph_builder.build())
            .expect("[Artisan] failed to insert Renderer State");
    };

    app.add_mut_system(frame_render.into_mut_system());
}

fn frame_render(world: &mut World, resources: &mut Resources) {
    let mut graph = resources
        .get_mut::<<ActiveContext as GpuContext>::Graph>()
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
