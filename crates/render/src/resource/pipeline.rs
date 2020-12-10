use crate::context::{CurrentContext, GpuContext};
use crate::resource::render_pass::SubpassId;
use crate::util::format::TextureFormat;
use std::borrow::Cow;
use std::fmt::Debug;
use std::mem::ManuallyDrop;
use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;

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
pub struct PipelineShaders {
    pub vertex: <CurrentContext as GpuContext>::ShaderCode,
    pub fragment: <CurrentContext as GpuContext>::ShaderCode,
    pub geometry: Option<<CurrentContext as GpuContext>::ShaderCode>, // etc...
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

#[derive(Debug, Clone)]
pub struct ViewportRect {
    x: i16,
    y: i16,
    width: i16,
    height: i16,
}

#[derive(Debug, Clone)]
pub struct Viewport {
    pub viewport: PipelineState<ViewportRect>,
    pub scissor: PipelineState<ViewportRect>,
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

#[derive(Debug)]
pub struct GraphicsPipelineDescriptor {
    pub name: Cow<'static, str>,

    pub shaders: PipelineShaders,
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
    pub viewport: Viewport,
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
