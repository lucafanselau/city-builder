use crate::context::GpuContext;
use crate::resource::render_pass::SubpassId;
use crate::util::format::TextureFormat;
use std::borrow::Cow;
use std::fmt::Debug;
use std::mem::ManuallyDrop;
use std::ops::{Deref, Range};
use std::path::Path;
use std::sync::Arc;

use super::glue::Mixture;

#[derive(Debug)]
pub enum ShaderSource {
    GlslFile(Cow<'static, Path>),
    GlslSource((&'static str, ShaderType, Option<&'static str>)),
    /// When used in compile shader this is a noop
    Spirv(Vec<u32>),
}

#[derive(Debug, Clone)]
pub enum ShaderType {
    Vertex,
    Fragment,
    Compute,
    Geometry,
}

#[derive(Debug)]
pub struct PipelineShaders<Context: GpuContext> {
    pub vertex: <Context as GpuContext>::ShaderCode,
    pub fragment: <Context as GpuContext>::ShaderCode,
    pub geometry: Option<<Context as GpuContext>::ShaderCode>, // etc...
}

#[derive(Debug, Clone)]
pub enum PolygonMode {
    Point,
    Line,
    Fill,
}

#[derive(Debug, Clone)]
pub enum Winding {
    Clockwise,
    CounterClockwise,
}

#[derive(Debug, Clone)]
pub enum CullFace {
    None,
    Front,
    Back,
}

#[derive(Debug, Clone)]
pub struct Culling {
    pub winding: Winding,
    pub cull_face: CullFace,
}

#[derive(Debug, Clone)]
pub struct Rasterizer {
    pub polygon_mode: PolygonMode,
    pub culling: Culling, // We might add fields later but for now this should be sufficent
}

impl Rasterizer {
    pub const FILL: Self = Rasterizer {
        polygon_mode: PolygonMode::Fill,
        culling: Culling {
            winding: Winding::Clockwise,
            cull_face: CullFace::None,
        },
    };
}

#[derive(Debug, Clone)]
pub enum VertexInputRate {
    Vertex,
    Instance,
}

#[derive(Debug, Clone)]
pub struct VertexBufferDescriptor {
    pub binding: u32,
    pub stride: u32,
    pub rate: VertexInputRate,
}

impl VertexBufferDescriptor {
    pub fn new(binding: u32, stride: u32, rate: VertexInputRate) -> Self {
        Self {
            binding,
            stride,
            rate,
        }
    }
}

#[derive(Debug, Clone)]
pub enum VertexAttributeFormat {
    Vec2,
    Vec3,
    Vec4,
}

#[derive(Debug, Clone)]
pub struct AttributeDescriptor {
    pub location: u32,
    /// index of the vertex buffer that fills this attribute
    pub binding: u32,
    pub offset: u32,
    pub format: VertexAttributeFormat,
}

impl AttributeDescriptor {
    pub fn new(location: u32, binding: u32, offset: u32, format: VertexAttributeFormat) -> Self {
        Self {
            location,
            binding,
            offset,
            format,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Primitive {
    PointList,
    LineList,
    LineStrip,
    TriangleList,
    TriangleStrip,
}

/// This is a copy from gfx_hal, so look there
#[derive(Debug, Clone)]
pub enum ComparisonFunction {
    Never,
    Less,
    Equal,
    LessEqual,
    Greater,
    NotEqual,
    GreaterEqual,
    Always,
}

#[derive(Debug, Clone)]
pub struct DepthDescriptor {
    pub function: ComparisonFunction,
    pub write: bool,
}

#[derive(Debug)]
pub enum PipelineState<T: Debug> {
    Baked(T),
    Dynamic,
}

impl<T: Clone + Debug> Clone for PipelineState<T> {
    fn clone(&self) -> Self {
        match self {
            PipelineState::Baked(t) => PipelineState::Baked(t.clone()),
            PipelineState::Dynamic => PipelineState::Dynamic,
        }
    }
}

impl<T: Debug + Clone> PipelineState<T> {
    pub fn to_option(&self) -> Option<T> {
        match self {
            PipelineState::Baked(t) => Some(t.clone()),
            PipelineState::Dynamic => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Rect {
    pub x: i16,
    pub y: i16,
    pub width: i16,
    pub height: i16,
}

#[derive(Debug, Clone)]
pub struct Viewport {
    pub rect: Rect,
    pub depth: Range<f32>,
}

#[derive(Debug, Clone)]
pub struct PipelineStates {
    pub viewport: PipelineState<Viewport>,
    pub scissor: PipelineState<Rect>,
}

#[derive(Debug, Clone)]
pub enum PipelineStage {
    TopOfPipe,
    DrawIndirect,
    VertexInput,
    VertexShader,
    HullShader,
    DomainShader,
    GeometryShader,
    FragmentShader,
    EarlyFragmentTests,
    LateFragmentTests,
    ColorAttachmentOutput,
    ComputeShader,
    Transfer,
    BottomOfPipe,
    Host,
    TaskShader,
    MeshShader,
}

// #[derive(Debug)]
// pub struct PipelineLayout<I>
// where
//     I: IntoIterator,
//     I::Item: Borrow<Mixture>,
// {
//     pub(crate) sets: I,
// }

#[derive(Debug)]
pub struct GraphicsPipelineDescriptor<'a, Context: GpuContext> {
    pub name: Cow<'static, str>,
    pub mixtures: Vec<&'a Mixture<Context>>,
    /// Push Constants
    pub push_constants: Vec<(ShaderType, Range<u32>)>,
    pub shaders: PipelineShaders<Context>,
    /// TODO: Render Pass layout for this Pipeline
    pub rasterizer: Rasterizer,
    // This should an probably will be serialized?
    /// List of expected vertex buffers for program execution
    pub vertex_buffers: Vec<VertexBufferDescriptor>,
    /// List of the Attributes that will be bound to the shader
    pub attributes: Vec<AttributeDescriptor>,
    /// the primitives that should be rendered
    pub primitive: Primitive,
    // TODO: Blending support, but since this is dependent on how we implement the render_pass
    //       system we might worry about it later, since alpha blending is not our primary goal
    //       right now :)
    /// A Vec representing if a color attachment should use alpha blending or not
    pub blend_targets: Vec<bool>,
    /// Enable a depth testing function
    pub depth: Option<DepthDescriptor>, // TODO: Multisampling
    /// The viewport for this pipeline
    pub pipeline_states: PipelineStates,
    // TODO: Descriptors? !!!!
}

#[derive(Debug)]
pub enum RenderContext<'a, Context: GpuContext + ?Sized> {
    RenderPass((&'a <Context as GpuContext>::RenderPassHandle, SubpassId)),
    /// Idea is that u can create a Pipeline without a specific render pass, by just giving a minimal
    /// set of attachments needed so a placeholder render_pass can be constructed that *should* be
    /// compatible to any real render pass that uses a super-set of those attachments
    ///
    /// Note: this is not properly implemented this is just here, so we can implement it later
    Attachments(&'a Vec<TextureFormat>),
}

#[derive(Debug)]
pub struct GraphicsPipeline<Context: GpuContext> {
    name: Cow<'static, str>,
    ctx: Arc<Context>,
    handle: ManuallyDrop<Context::PipelineHandle>,
}

impl<Context: GpuContext> GraphicsPipeline<Context> {
    pub fn new(
        name: Cow<'static, str>,
        handle: Context::PipelineHandle,
        ctx: Arc<Context>,
    ) -> Self {
        Self {
            name,
            ctx,
            handle: ManuallyDrop::new(handle),
        }
    }

    pub fn get_handle(&self) -> &Context::PipelineHandle {
        self.handle.deref()
    }
}

impl<Context: GpuContext> Deref for GraphicsPipeline<Context> {
    type Target = Context::PipelineHandle;

    fn deref(&self) -> &Self::Target {
        self.handle.deref()
    }
}

impl<Context: GpuContext> Drop for GraphicsPipeline<Context> {
    fn drop(&mut self) {
        unsafe {
            self.ctx.drop_pipeline(ManuallyDrop::take(&mut self.handle));
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::command_encoder::*;
    use crate::context::GpuContext;
    use crate::replay::*;
    use crate::resource::{buffer::*, frame::*, pipeline::*, render_pass::*};
    use crate::util::format::*;
    use bytemuck::{Pod, Zeroable};
    use log::*;
    use std::ops::Deref;

    #[derive(Copy, Clone, Zeroable, Pod)]
    #[repr(C)]
    struct Vertex {
        pos: [f32; 4],
    }

    const VERTEX_CODE: &str = r#"
    #version 450
    layout (location = 0) in vec4 in_pos;
    void main() { gl_Position = in_pos; }
    "#;

    const FRAGMENT_CODE: &str = r#"
    #version 450
    #extension GL_ARB_separate_shader_objects : enable
    layout(location = 0) out vec4 outColor;
    void main() { outColor = vec4(1.0, 0.0, 0.0, 1.0); }
    "#;

    #[test]
    fn simple_pipeline() {
        let (ctx, resources, window, event_loop) = create_context();
        let extent = (1600, 900);

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

        let vertex_buffer = resources.create_empty_buffer(BufferDescriptor {
            name: "Simple Vertex Buffer".into(),
            size: (vertex_size * vertices.len()) as u64,
            memory_type: MemoryType::HostVisible,
            usage: BufferUsage::Vertex,
        });

        unsafe {
            ctx.write_to_buffer(vertex_buffer.deref(), &vertices);
        }

        let (pipeline, render_pass) = {
            let vertex_code = ctx.compile_shader(ShaderSource::GlslSource((
                VERTEX_CODE,
                ShaderType::Vertex,
                Some("test_vertex_shader"),
            )));
            let fragment_code = ctx.compile_shader(ShaderSource::GlslSource((
                FRAGMENT_CODE,
                ShaderType::Fragment,
                Some("test_fragment_shader"),
            )));

            let render_pass = {
                let color_attachment = Attachment {
                    format: ctx.get_surface_format(),
                    load_op: LoadOp::Clear,
                    store_op: StoreOp::Store,
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

            let pipeline = resources
                .create_graphics_pipeline(&desc, RenderContext::RenderPass((&render_pass, 0)));

            (pipeline, render_pass)
        };

        run_loop(window, event_loop, move || {
            let swapchain_image = ctx.new_frame();

            use std::borrow::Borrow;

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

                cmd.draw(0..3, 0..1);

                cmd.end_render_pass();
            });

            ctx.end_frame(swapchain_image, frame_commands);

            ctx.drop_framebuffer(framebuffer);
        })
    }
}
