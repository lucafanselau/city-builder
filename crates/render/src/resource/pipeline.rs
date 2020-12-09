use crate::context::{CurrentContext, GpuContext};
use std::borrow::Cow;
use std::fmt::Debug;
use std::mem::ManuallyDrop;
use std::ops::Deref;
use std::sync::Arc;

#[derive(Debug)]
pub struct PipelineShaders {
    pub vertex: Vec<u32>,
    pub fragment: Vec<u32>,
    pub geometry: Option<Vec<u32>>, // etc...
}

#[derive(Debug)]
pub(crate) enum PolygonMode {
    Point,
    Line,
    Fill,
}

#[derive(Debug)]
pub(crate) enum Winding {
    Clockwise,
    CounterClockwise,
}

#[derive(Debug)]
pub(crate) enum CullFace {
    None,
    Front,
    Back,
}

#[derive(Debug)]
pub(crate) struct Culling {
    pub winding: Winding,
    pub cull_face: CullFace,
}

#[derive(Debug)]
pub(crate) struct Rasterizer {
    pub polygon_mode: PolygonMode,
    pub culling: Culling, // We might add fields later but for now this should be sufficent
}

#[derive(Debug, Clone)]
pub(crate) enum VertexInputRate {
    Vertex,
    Instance,
}

#[derive(Debug, Clone)]
pub(crate) struct VertexBufferDescriptor {
    pub binding: u32,
    pub stride: u32,
    pub rate: VertexInputRate,
}

#[derive(Debug)]
pub(crate) enum VertexAttributeFormat {
    Vec2,
    Vec3,
    Vec4,
}

#[derive(Debug)]
pub(crate) struct AttributeDescriptor {
    pub location: u32,
    /// index of the vertex buffer that fills this attribute
    pub binding: u32,
    pub offset: u32,
    pub format: VertexAttributeFormat,
}

#[derive(Debug)]
pub(crate) enum Primitive {
    PointList,
    LineList,
    LineStrip,
    TriangleList,
    TriangleStrip,
}

/// This is a copy from gfx_hal, so look there
#[derive(Debug, Clone)]
pub(crate) enum ComparisonFunction {
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
pub(crate) struct DepthDescriptor {
    pub function: ComparisonFunction,
    pub write: bool,
}

#[derive(Debug)]
pub(crate) enum PipelineState<T: Debug> {
    Baked(T),
    Dynamic,
}

#[derive(Debug)]
pub(crate) struct ViewportRect {
    x: i16,
    y: i16,
    width: i16,
    height: i16,
}

#[derive(Debug)]
pub(crate) struct Viewport {
    viewport: PipelineState<ViewportRect>,
    scissor: PipelineState<ViewportRect>,
}

#[derive(Debug)]
pub struct GraphicsPipelineDescriptor {
    pub(crate) name: Cow<'static, str>,

    pub(crate) shaders: PipelineShaders,
    /// TODO: Render Pass layout for this Pipeline
    pub(crate) rasterizer: Rasterizer,
    // This should an probably will be serialized?
    /// List of expected vertex buffers for program execution
    pub(crate) vertex_buffers: Vec<VertexBufferDescriptor>,
    /// List of the Attributes that will be bound to the shader
    pub(crate) attributes: Vec<AttributeDescriptor>,
    /// the primitives that should be rendered
    pub(crate) primitive: Primitive,
    // TODO: Blending support, but since this is dependent on how we implement the render_pass
    //       system we might worry about it later, since alpha blending is not our primary goal
    //       right now :)
    /// Enable a depth testing function
    pub(crate) depth: Option<DepthDescriptor>, // TODO: Multisampling
    /// The viewport for this pipeline
    pub(crate) viewport: Viewport,
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
