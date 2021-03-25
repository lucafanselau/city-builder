use std::{cell::Ref, ops::Deref, path::Path, sync::Arc};

use app::{App, AssetDescendant, IntoMutatingSystem, Resources, Timing, World};
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
            DepthDescriptor, GraphicsPipeline, PipelineShaders, PipelineStates, Primitive,
            Rasterizer, RenderContext as PipelineRenderContext,
        },
        render_pass::{LoadOp, StoreOp},
    },
};

use crate::{
    camera::{Camera, CameraBuffer},
    components::{MeshComponent, Transform},
    material::{MaterialComponent, SolidMaterial},
    mesh::{MeshMap, Vertex},
    shader_assets::ShaderAsset,
};

const LIGHT_POSITION: Vec3A = glam::const_vec3a!([10.0, 10.0, 10.0]);
const MAT4_SIZE: u32 = std::mem::size_of::<glam::Mat4>() as _;
const MATERIAL_SIZE: u32 = std::mem::size_of::<SolidMaterial>() as _;

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

    // Initialize shader asset
    crate::shader_assets::init(app, ctx.clone());

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

        let mixture = Arc::new(resources.stir(parts));

        // TODO: load that from file or something
        let vertex_shader = app
            .load_asset("assets/shaders/solid.vert")
            .expect("failed to load solid vertex");
        let fragment_shader = app
            .load_asset("assets/shaders/solid.frag")
            .expect("failed to load solid fragment");

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
            let mut builder = graph_builder.build_pass_node("main_pass".into());
            builder.add_output(backbuffer, LoadOp::Clear, StoreOp::Store);
            builder.set_depth(depth_attachment, LoadOp::Clear, StoreOp::DontCare);
            let resources = resources.clone();
            builder.init(Box::new(move |render_pass| {
                let resources = resources.clone();
                let vertex_shader = vertex_shader.clone();
                let fragment_shader = fragment_shader.clone();
                let mixture = mixture.clone();
                let pipeline = AssetDescendant::<ShaderAsset, _>::new(
                    move |assets| {
                        let (buffer_descriptor, attributes) = Vertex::get_layout();
                        let desc = GraphicsPipelineDescriptor {
                            name: "simple_pipeline".into(),
                            mixtures: vec![&mixture],
                            push_constants: vec![
                                (ShaderType::Vertex, 0..MAT4_SIZE),
                                (ShaderType::Fragment, MAT4_SIZE..MAT4_SIZE + MATERIAL_SIZE),
                            ],
                            shaders: PipelineShaders {
                                vertex: &assets[0].0,
                                fragment: &assets[1].0,
                                geometry: None,
                            },
                            rasterizer: Rasterizer::FILL,
                            vertex_buffers: vec![buffer_descriptor],
                            attributes,
                            primitive: Primitive::TriangleList,
                            blend_targets: vec![true],
                            depth: Some(DepthDescriptor::LESS),
                            pipeline_states: PipelineStates::DYNAMIC,
                        };

                        resources.create_graphics_pipeline(
                            desc,
                            PipelineRenderContext::RenderPass((render_pass.deref(), 0)),
                        )
                    },
                    vec![vertex_shader, fragment_shader].as_slice(),
                );

                Box::new(pipeline)
            }));
            builder.callback(Box::new(move |frame, pipeline, world, resources| {
                if let Some(pipeline) = pipeline.get(resources) {
                    let FrameData {
                        cmd,
                        frame_index,
                        viewport,
                    } = frame;

                    {
                        // Query needed resources
                        let (camera, timing): (Ref<Camera>, Ref<Timing>) =
                            resources.query::<(Ref<Camera>, Ref<Timing>)>()?;
                        // Calculate new camera
                        let camera_data = {
                            let aspect_ratio =
                                viewport.rect.width as f32 / viewport.rect.height as f32;
                            camera.calc(aspect_ratio)
                        };
                        camera_buffer.write(camera_data);
                        camera_buffer.frame(frame_index);
                        // Update Light Buffer
                        let light_position = glam::vec3a(
                            timing.total_elapsed().sin() * 10.0,
                            10.0,
                            timing.total_elapsed().cos() * 10.0,
                        );
                        let light_data = Light {
                            light_position,
                            view_position: camera.eye.into(),
                        };
                        light_buffer.write(light_data);
                        light_buffer.frame(frame_index);
                    }

                    let mesh_map = resources
                        .get::<MeshMap>()
                        .expect("[Artisan] (renderer) failed to get mesh map");

                    cmd.bind_graphics_pipeline(&pipeline);

                    // TODO: Viewport
                    cmd.set_viewport(0, viewport.clone());
                    cmd.set_scissor(0, viewport.rect);

                    cmd.snort_glue(0, &pipeline, &glue_drops[frame_index as usize]);

                    for (_e, (mesh, mat, transform)) in world
                        .query::<(&MeshComponent, &MaterialComponent, &Transform)>()
                        .iter()
                    {
                        if let MaterialComponent::Solid(ref solid) = mat {
                            let (vertex_count, buffer) = mesh_map.draw_info(&mesh.0);

                            let model = transform.into_model();
                            let vertex_push_data: &[u32] =
                                bytemuck::cast_slice(bytemuck::bytes_of(&model));
                            cmd.push_constants(&pipeline, ShaderType::Vertex, 0, vertex_push_data);

                            let fragment_push_data: &[u32] =
                                bytemuck::cast_slice(bytemuck::bytes_of(solid));
                            // log::info!("Push Data is: \n{:#?}", push_data);
                            cmd.push_constants(
                                &pipeline,
                                ShaderType::Fragment,
                                MAT4_SIZE,
                                fragment_push_data,
                            );

                            cmd.bind_vertex_buffer(0, buffer, BufferRange::WHOLE);
                            cmd.draw(0..vertex_count, 0..1);
                        } else {
                            unimplemented!();
                        }
                    }
                }
                Ok(())
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
}
