use crate::resource::pipeline::{
    AttributeDescriptor, ComparisonFunction, CullFace, DepthDescriptor, PolygonMode, Primitive,
    Rasterizer, VertexAttributeFormat, VertexBufferDescriptor, VertexInputRate, Winding,
};
use crate::util::format::TextureFormat;
use gfx_hal::format::Format;
use gfx_hal::pso::{
    AttributeDesc, DepthTest, Element, Face, FrontFace, PolygonMode as HalPolygonMode,
    Primitive as HalPrimitive, Rasterizer as HalRasterizer, State, VertexBufferDesc,
    VertexInputRate as HalInputRate,
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
