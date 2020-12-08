use crate::resource::pipeline::{
    AttributeDescriptor, CullFace, PolygonMode, Primitive, Rasterizer, VertexAttributeFormat,
    VertexBufferDescriptor, VertexInputRate, Winding,
};
use gfx_hal::format::Format;
use gfx_hal::pso::{
    AttributeDesc, Element, Face, FrontFace, PolygonMode as HalPolygonMode,
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
