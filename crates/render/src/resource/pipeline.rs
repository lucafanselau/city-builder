use crate::context::{CurrentContext, GpuContext};
use std::borrow::Cow;
use std::fmt::Debug;
use std::mem::ManuallyDrop;
use std::ops::Deref;
use std::sync::Arc;

#[derive(Debug)]
pub struct PipelineShaders {
    vertex: Vec<u32>,
    fragment: Vec<u32>,
    geometry: Option<Vec<u32>>, // etc...
}

#[derive(Debug)]
enum PolygonMode {
    Point,
    Line,
    Fill,
}

#[derive(Debug)]
enum Winding {
    Clockwise,
    CounterClockwise,
}

#[derive(Debug)]
enum CullFace {
    Front,
    Back,
}

#[derive(Debug)]
struct Culling {
    winding: Winding,
    cull_face: CullFace,
}

#[derive(Debug)]
struct Rasterizer {
    polygon_mode: PolygonMode,
    culling: Culling, // We might add fields later but for now this should be sufficent
}

#[derive(Debug)]
enum VertexInputRate {
    Vertex,
    Instance,
}

#[derive(Debug)]
struct VertexBufferDescriptor {
    binding: u32,
    stride: u32,
    rate: VertexInputRate,
}

#[derive(Debug)]
enum VertexAttributeFormat {
    Vec2,
    Vec3,
    Vec4,
}

#[derive(Debug)]
struct AttributeDescriptor {
    location: u32,
    /// index of the vertex buffer that fills this attribute
    binding: u32,
    offset: u32,
    format: VertexAttributeFormat,
}

#[derive(Debug)]
enum Primitive {
    PointList,
    LineList,
    LineStrip,
    TriangleList,
    TriangleStrip,
}

/// This is a copy from gfx_hal, so look there
#[derive(Debug)]
enum ComparisonFunction {
    Never,
    Less,
    Equal,
    LessEqual,
    Greater,
    NotEqual,
    GreaterEqual,
    Always,
}

#[derive(Debug)]
struct DepthDescriptor {
    function: ComparisonFunction,
    write: bool,
}

#[derive(Debug)]
enum PipelineState<T: Debug> {
    Baked(T),
    Dynamic,
}

#[derive(Debug)]
struct ViewportRect {
    x: i16,
    y: i16,
    width: i16,
    height: i16,
}

#[derive(Debug)]
struct Viewport {
    viewport: PipelineState<ViewportRect>,
    scissor: PipelineState<ViewportRect>,
}

#[derive(Debug)]
pub struct GraphicsPipelineDescriptor {
    pub(crate) name: Cow<'static, str>,

    shaders: PipelineShaders,
    /// TODO: Render Pass layout for this Pipeline
    rasterizer: Rasterizer,
    // This should an probably will be serialized?
    /// List of expected vertex buffers for program execution
    vertex_buffers: Vec<VertexBufferDescriptor>,
    /// List of the Attributes that will be bound to the shader
    attributes: Vec<AttributeDescriptor>,
    /// the primitives that should be rendered
    primitive: Primitive,
    // TODO: Blending support, but since this is dependent on how we implement the render_pass
    //       system we might worry about it later, since alpha blending is not our primary goal
    //       right now :)
    /// Enable a depth testing function
    depth: Option<DepthDescriptor>, // TODO: Multisampling
    /// The viewport for this pipeline
    viewport: Viewport,
    // TODO: Descriptors? !!!!
}

type PipelineHandle = <CurrentContext as GpuContext>::PipelineHandle;

#[derive(Debug)]
pub struct GraphicsPipeline {
    name: Cow<'static, str>,
    ctx: Arc<CurrentContext>,
    handle: ManuallyDrop<PipelineHandle>,
}

impl GraphicsPipeline {
    pub fn new(name: Cow<'static, str>, handle: PipelineHandle, ctx: Arc<CurrentContext>) -> Self {
        Self {
            name,
            ctx,
            handle: ManuallyDrop::new(handle),
        }
    }

    pub fn get_handle(&self) -> &PipelineHandle {
        self.handle.deref()
    }
}

impl Deref for GraphicsPipeline {
    type Target = PipelineHandle;

    fn deref(&self) -> &Self::Target {
        self.handle.deref()
    }
}

impl Drop for GraphicsPipeline {
    fn drop(&mut self) {
        unsafe {
            self.ctx.drop_pipeline(ManuallyDrop::take(&mut self.handle));
        }
    }
}
