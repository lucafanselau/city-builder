use gfx_hal::image::{Access, Extent, Layout};
use gfx_hal::memory::Dependencies;
use gfx_hal::pass::{
    Attachment as HalAttachment, AttachmentId, AttachmentLoadOp as HalAttachmentLoadOp,
    AttachmentOps, AttachmentRef as HalAttachmentRef, AttachmentStoreOp as HalAttachmentStoreOp,
    SubpassDependency as HalSubpassDependency,
};
use gfx_hal::pso::{
    AttributeDesc, DepthTest, Element, Face, FrontFace, PipelineStage as HalPipelineStage,
    PolygonMode as HalPolygonMode, Primitive as HalPrimitive, Rasterizer as HalRasterizer,
    Rect as HalRect, State, VertexBufferDesc, VertexInputRate as HalInputRate,
    Viewport as HalViewport,
};
use gfx_hal::{
    buffer::SubRange,
    pso::{DescriptorSetLayoutBinding, ShaderStageFlags},
    window::Extent2D as HalExtent2D,
    IndexType as HalIndexType,
};
use gfx_hal::{
    command::{BufferCopy as HalBufferCopy, ClearColor, ClearDepthStencil, ClearValue},
    pso::DescriptorType,
};
use gfx_hal::{format::Format, image::Tiling};
use render::resource::{
    buffer::BufferCopy,
    render_pass::{
        Attachment, AttachmentRef, LoadOp, StoreOp, SubpassDependency, SubpassDescriptor,
    },
};
use render::resource::{buffer::BufferRange, glue::MixturePart, pipeline::ShaderType};
use render::util::format::{ImageAccess, TextureFormat, TextureLayout};
use render::{
    command_encoder::IndexType,
    resource::pipeline::{
        AttributeDescriptor, ComparisonFunction, CullFace, DepthDescriptor, PipelineStage,
        PolygonMode, Primitive, Rasterizer, Rect, VertexAttributeFormat, VertexBufferDescriptor,
        VertexInputRate, Viewport, Winding,
    },
};
use render::{
    resource::frame::{Clear, Extent2D, Extent3D},
    util::format::ImageTiling,
};

pub trait ToHalType {
    type Target;
    fn convert(self) -> Self::Target;
}

impl ToHalType for Primitive {
    type Target = HalPrimitive;

    fn convert(self) -> HalPrimitive {
        match self {
            Primitive::LineList => HalPrimitive::LineList,
            Primitive::PointList => HalPrimitive::PointList,
            Primitive::LineStrip => HalPrimitive::LineStrip,
            Primitive::TriangleList => HalPrimitive::TriangleList,
            Primitive::TriangleStrip => HalPrimitive::TriangleStrip,
        }
    }
}

impl ToHalType for VertexBufferDescriptor {
    type Target = VertexBufferDesc;

    fn convert(self) -> Self::Target {
        VertexBufferDesc {
            binding: self.binding,
            stride: self.stride,
            rate: match self.rate {
                VertexInputRate::Vertex => HalInputRate::Vertex,
                VertexInputRate::Instance => HalInputRate::Instance(1),
            },
        }
    }
}

impl ToHalType for AttributeDescriptor {
    type Target = AttributeDesc;

    fn convert(self) -> Self::Target {
        let format = match self.format {
            VertexAttributeFormat::Vec2 => Format::Rg32Sfloat,
            VertexAttributeFormat::Vec3 => Format::Rgb32Sfloat,
            VertexAttributeFormat::Vec4 => Format::Rgba32Sfloat,
        };

        AttributeDesc {
            location: self.location,
            binding: self.binding,
            element: Element {
                format,
                offset: self.offset,
            },
        }
    }
}

impl ToHalType for PolygonMode {
    type Target = HalPolygonMode;

    fn convert(self) -> Self::Target {
        match self {
            PolygonMode::Point => HalPolygonMode::Point,
            PolygonMode::Line => HalPolygonMode::Line,
            PolygonMode::Fill => HalPolygonMode::Fill,
        }
    }
}

impl ToHalType for CullFace {
    type Target = Face;

    fn convert(self) -> Self::Target {
        match self {
            CullFace::None => Face::NONE,
            CullFace::Front => Face::FRONT,
            CullFace::Back => Face::BACK,
        }
    }
}

impl ToHalType for Winding {
    type Target = FrontFace;

    fn convert(self) -> Self::Target {
        match self {
            Winding::Clockwise => FrontFace::Clockwise,
            Winding::CounterClockwise => FrontFace::CounterClockwise,
        }
    }
}

impl ToHalType for Rasterizer {
    type Target = HalRasterizer;

    fn convert(self) -> Self::Target {
        HalRasterizer {
            polygon_mode: self.polygon_mode.convert(),
            cull_face: self.culling.cull_face.convert(),
            front_face: self.culling.winding.convert(),
            depth_clamping: false,
            depth_bias: None,
            conservative: false,
            line_width: State::Static(1.0),
        }
    }
}

impl ToHalType for DepthDescriptor {
    type Target = DepthTest;

    fn convert(self) -> Self::Target {
        use gfx_hal::pso::Comparison;

        let fun = match self.function {
            ComparisonFunction::Never => Comparison::Never,
            ComparisonFunction::Less => Comparison::Less,
            ComparisonFunction::Equal => Comparison::Equal,
            ComparisonFunction::LessEqual => Comparison::LessEqual,
            ComparisonFunction::Greater => Comparison::Greater,
            ComparisonFunction::NotEqual => Comparison::NotEqual,
            ComparisonFunction::GreaterEqual => Comparison::GreaterEqual,
            ComparisonFunction::Always => Comparison::Always,
        };

        DepthTest {
            fun,
            write: self.write,
        }
    }
}

impl ToHalType for TextureFormat {
    type Target = Format;

    fn convert(self) -> Self::Target {
        match self {
            TextureFormat::R8Unorm => Format::R8Unorm,
            TextureFormat::R8Snorm => Format::R8Snorm,
            TextureFormat::R8Uint => Format::R8Uint,
            TextureFormat::R8Sint => Format::R8Sint,
            TextureFormat::R16Uint => Format::R16Uint,
            TextureFormat::R16Sint => Format::R16Sint,
            TextureFormat::R16Sfloat => Format::R16Sfloat,
            TextureFormat::Rg8Unorm => Format::Rg8Unorm,
            TextureFormat::Rg8Snorm => Format::Rg8Snorm,
            TextureFormat::Rg8Uint => Format::Rg8Uint,
            TextureFormat::Rg8Sint => Format::Rg8Sint,
            TextureFormat::R32Uint => Format::R32Uint,
            TextureFormat::R32Sint => Format::R32Sint,
            TextureFormat::R32Sfloat => Format::R32Sfloat,
            TextureFormat::Rg16Uint => Format::Rg16Uint,
            TextureFormat::Rg16Sint => Format::Rg16Sint,
            TextureFormat::Rg16Sfloat => Format::Rg16Sfloat,
            TextureFormat::Rgba8Unorm => Format::Rgba8Unorm,
            TextureFormat::Rgba8Snorm => Format::Rgba8Snorm,
            TextureFormat::Rgba8Srgb => Format::Rgba8Srgb,
            TextureFormat::Rgba8Uint => Format::Rgba8Uint,
            TextureFormat::Rgba8Sint => Format::Rgba8Sint,
            TextureFormat::Bgra8Unorm => Format::Bgra8Unorm,
            TextureFormat::Bgra8Srgb => Format::Bgra8Srgb,
            TextureFormat::Rg32Uint => Format::Rg32Uint,
            TextureFormat::Rg32Sint => Format::Rg32Sint,
            TextureFormat::Rg32Sfloat => Format::Rg32Sfloat,
            TextureFormat::Rgba16Uint => Format::Rgba16Uint,
            TextureFormat::Rgba16Sint => Format::Rgba16Sint,
            TextureFormat::Rgba16Sfloat => Format::Rgba16Sfloat,
            TextureFormat::Rgba32Uint => Format::Rgba32Uint,
            TextureFormat::Rgba32Sint => Format::Rgba32Sint,
            TextureFormat::Rgba32Sfloat => Format::Rgba32Sfloat,
            TextureFormat::Depth32Sfloat => Format::D32Sfloat,
            TextureFormat::Depth24PlusStencil8 => Format::D24UnormS8Uint,
        }
    }
}

impl ToHalType for TextureLayout {
    type Target = Layout;

    fn convert(self) -> Self::Target {
        match self {
            TextureLayout::General => Layout::General,
            TextureLayout::ColorAttachmentOptimal => Layout::ColorAttachmentOptimal,
            TextureLayout::DepthStencilAttachmentOptimal => Layout::DepthStencilAttachmentOptimal,
            TextureLayout::DepthStencilReadOnlyOptimal => Layout::DepthStencilReadOnlyOptimal,
            TextureLayout::ShaderReadOnlyOptimal => Layout::ShaderReadOnlyOptimal,
            TextureLayout::TransferSrcOptimal => Layout::TransferSrcOptimal,
            TextureLayout::TransferDstOptimal => Layout::TransferDstOptimal,
            TextureLayout::Undefined => Layout::Undefined,
            TextureLayout::Preinitialized => Layout::Preinitialized,
            TextureLayout::Present => Layout::Present,
        }
    }
}

impl ToHalType for LoadOp {
    type Target = HalAttachmentLoadOp;

    fn convert(self) -> Self::Target {
        match self {
            LoadOp::Load => HalAttachmentLoadOp::Load,
            LoadOp::Clear => HalAttachmentLoadOp::Clear,
            LoadOp::DontCare => HalAttachmentLoadOp::DontCare,
        }
    }
}

impl ToHalType for StoreOp {
    type Target = HalAttachmentStoreOp;

    fn convert(self) -> Self::Target {
        match self {
            StoreOp::Store => HalAttachmentStoreOp::Store,
            StoreOp::DontCare => HalAttachmentStoreOp::DontCare,
        }
    }
}

impl ToHalType for PipelineStage {
    type Target = HalPipelineStage;

    fn convert(self) -> Self::Target {
        match self {
            PipelineStage::TopOfPipe => HalPipelineStage::TOP_OF_PIPE,
            PipelineStage::DrawIndirect => HalPipelineStage::DRAW_INDIRECT,
            PipelineStage::VertexInput => HalPipelineStage::VERTEX_INPUT,
            PipelineStage::VertexShader => HalPipelineStage::VERTEX_SHADER,
            PipelineStage::HullShader => HalPipelineStage::HULL_SHADER,
            PipelineStage::DomainShader => HalPipelineStage::DOMAIN_SHADER,
            PipelineStage::GeometryShader => HalPipelineStage::GEOMETRY_SHADER,
            PipelineStage::FragmentShader => HalPipelineStage::FRAGMENT_SHADER,
            PipelineStage::EarlyFragmentTests => HalPipelineStage::EARLY_FRAGMENT_TESTS,
            PipelineStage::LateFragmentTests => HalPipelineStage::LATE_FRAGMENT_TESTS,
            PipelineStage::ColorAttachmentOutput => HalPipelineStage::COLOR_ATTACHMENT_OUTPUT,
            PipelineStage::ComputeShader => HalPipelineStage::COMPUTE_SHADER,
            PipelineStage::Transfer => HalPipelineStage::TRANSFER,
            PipelineStage::BottomOfPipe => HalPipelineStage::BOTTOM_OF_PIPE,
            PipelineStage::Host => HalPipelineStage::HOST,
            PipelineStage::TaskShader => HalPipelineStage::TASK_SHADER,
            PipelineStage::MeshShader => HalPipelineStage::MESH_SHADER,
        }
    }
}

impl ToHalType for ImageAccess {
    type Target = Access;

    fn convert(self) -> Self::Target {
        match self {
            ImageAccess::InputAttachmentRead => Access::INPUT_ATTACHMENT_READ,
            ImageAccess::ShaderRead => Access::SHADER_READ,
            ImageAccess::ShaderWrite => Access::SHADER_WRITE,
            ImageAccess::ColorAttachmentRead => Access::COLOR_ATTACHMENT_READ,
            ImageAccess::ColorAttachmentWrite => Access::COLOR_ATTACHMENT_WRITE,
            ImageAccess::DepthStencilAttachmentRead => Access::DEPTH_STENCIL_ATTACHMENT_READ,
            ImageAccess::DepthStencilAttachmentWrite => Access::DEPTH_STENCIL_ATTACHMENT_WRITE,
            ImageAccess::TransferRead => Access::TRANSFER_READ,
            ImageAccess::TransferWrite => Access::TRANSFER_WRITE,
            ImageAccess::HostRead => Access::HOST_READ,
            ImageAccess::HostWrite => Access::HOST_WRITE,
            ImageAccess::MemoryRead => Access::MEMORY_READ,
            ImageAccess::MemoryWrite => Access::MEMORY_WRITE,
        }
    }
}

impl ToHalType for Attachment {
    type Target = HalAttachment;

    fn convert(self) -> Self::Target {
        HalAttachment {
            format: Some(self.format.convert()),
            samples: 1,
            ops: AttachmentOps::new(self.load_op.convert(), self.store_op.convert()),
            stencil_ops: AttachmentOps::DONT_CARE,
            layouts: self.layouts.start.convert()..self.layouts.end.convert(),
        }
    }
}

impl ToHalType for AttachmentRef {
    type Target = HalAttachmentRef;

    fn convert(self) -> Self::Target {
        (self.0, self.1.convert())
    }
}

pub struct HalCompatibleSubpassDescriptor {
    pub colors: Vec<HalAttachmentRef>,
    pub depth_stencil: Option<HalAttachmentRef>,
    pub inputs: Vec<HalAttachmentRef>,
    pub resolves: Vec<HalAttachmentRef>,
    pub preserves: Vec<AttachmentId>,
}

impl ToHalType for SubpassDescriptor {
    type Target = HalCompatibleSubpassDescriptor;

    fn convert(self) -> Self::Target {
        HalCompatibleSubpassDescriptor {
            colors: self.colors.into_iter().map(|a| a.convert()).collect(),
            depth_stencil: self.depth_stencil.map(|a| a.convert()),
            inputs: self.inputs.into_iter().map(|a| a.convert()).collect(),
            resolves: self.resolves.into_iter().map(|a| a.convert()).collect(),
            preserves: self.preserves,
        }
    }
}

impl ToHalType for SubpassDependency {
    type Target = HalSubpassDependency;

    fn convert(self) -> Self::Target {
        HalSubpassDependency {
            passes: self.passes,
            stages: self.stages.start.convert()..self.stages.end.convert(),
            accesses: self.accesses.start.convert()..self.accesses.end.convert(),
            flags: Dependencies::empty(),
        }
    }
}

impl ToHalType for Extent2D {
    type Target = HalExtent2D;

    fn convert(self) -> Self::Target {
        HalExtent2D {
            width: self.width,
            height: self.height,
        }
    }
}

impl ToHalType for Extent3D {
    type Target = Extent;

    fn convert(self) -> Self::Target {
        Extent {
            width: self.width,
            height: self.height,
            depth: self.depth,
        }
    }
}

impl ToHalType for Rect {
    type Target = HalRect;

    fn convert(self) -> Self::Target {
        HalRect {
            x: self.x,
            y: self.y,
            w: self.width,
            h: self.height,
        }
    }
}

impl ToHalType for Viewport {
    type Target = HalViewport;

    fn convert(self) -> Self::Target {
        HalViewport {
            rect: self.rect.convert(),
            depth: self.depth,
        }
    }
}

impl ToHalType for Clear {
    type Target = ClearValue;

    fn convert(self) -> Self::Target {
        match self {
            Clear::Color(r, g, b, a) => ClearValue {
                color: ClearColor {
                    float32: [r, g, b, a],
                },
            },
            Clear::Depth(d, s) => ClearValue {
                depth_stencil: ClearDepthStencil {
                    depth: d,
                    stencil: s,
                },
            },
        }
    }
}

impl ToHalType for BufferRange {
    type Target = SubRange;

    fn convert(self) -> Self::Target {
        SubRange {
            offset: self.offset,
            size: self.size,
        }
    }
}

impl ToHalType for ShaderType {
    type Target = ShaderStageFlags;

    fn convert(self) -> Self::Target {
        match self {
            ShaderType::Vertex => ShaderStageFlags::VERTEX,
            ShaderType::Fragment => ShaderStageFlags::FRAGMENT,
            ShaderType::Compute => ShaderStageFlags::COMPUTE,
            ShaderType::Geometry => ShaderStageFlags::GEOMETRY,
        }
    }
}

pub(in crate) fn get_descriptor_type(part: &MixturePart) -> DescriptorType {
    match part.type_info {
        render::resource::glue::PartType::Uniform(_) => DescriptorType::Buffer {
            ty: gfx_hal::pso::BufferDescriptorType::Uniform,
            format: gfx_hal::pso::BufferDescriptorFormat::Structured {
                dynamic_offset: part.is_dynamic,
            },
        },
        render::resource::glue::PartType::Sampler => DescriptorType::Image {
            ty: gfx_hal::pso::ImageDescriptorType::Sampled { with_sampler: true },
        },
    }
}

impl ToHalType for MixturePart {
    type Target = DescriptorSetLayoutBinding;

    fn convert(self) -> Self::Target {
        DescriptorSetLayoutBinding {
            binding: self.binding,
            ty: get_descriptor_type(&self),
            count: self.array_size,
            stage_flags: self.shader_type.convert(),
            immutable_samplers: false,
        }
    }
}

impl ToHalType for BufferCopy {
    type Target = HalBufferCopy;

    fn convert(self) -> Self::Target {
        HalBufferCopy {
            src: self.src_offset,
            dst: self.dst_offset,
            size: self.size,
        }
    }
}

impl ToHalType for ImageTiling {
    type Target = Tiling;

    fn convert(self) -> Self::Target {
        match self {
            ImageTiling::Linear => Tiling::Linear,
            ImageTiling::Optimal => Tiling::Optimal,
        }
    }
}

impl ToHalType for IndexType {
    type Target = HalIndexType;

    fn convert(self) -> Self::Target {
        match self {
            IndexType::U16 => HalIndexType::U16,
            IndexType::U32 => HalIndexType::U32,
        }
    }
}
