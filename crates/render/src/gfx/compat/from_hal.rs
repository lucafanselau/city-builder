use crate::util::format::TextureFormat;
use gfx_hal::format::Format;

pub trait FromHalType {
    type Target;
    fn convert(self) -> Result<Self::Target, String>;
}

impl FromHalType for Format {
    type Target = TextureFormat;

    fn convert(self) -> Result<Self::Target, String> {
        match self {
            Format::R8Unorm => Ok(TextureFormat::R8Unorm),
            Format::R8Snorm => Ok(TextureFormat::R8Snorm),
            Format::R8Uint => Ok(TextureFormat::R8Uint),
            Format::R8Sint => Ok(TextureFormat::R8Sint),
            Format::R16Uint => Ok(TextureFormat::R16Uint),
            Format::R16Sint => Ok(TextureFormat::R16Sint),
            Format::R16Sfloat => Ok(TextureFormat::R16Sfloat),
            Format::Rg8Unorm => Ok(TextureFormat::Rg8Unorm),
            Format::Rg8Snorm => Ok(TextureFormat::Rg8Snorm),
            Format::Rg8Uint => Ok(TextureFormat::Rg8Uint),
            Format::Rg8Sint => Ok(TextureFormat::Rg8Sint),
            Format::R32Uint => Ok(TextureFormat::R32Uint),
            Format::R32Sint => Ok(TextureFormat::R32Sint),
            Format::R32Sfloat => Ok(TextureFormat::R32Sfloat),
            Format::Rg16Uint => Ok(TextureFormat::Rg16Uint),
            Format::Rg16Sint => Ok(TextureFormat::Rg16Sint),
            Format::Rg16Sfloat => Ok(TextureFormat::Rg16Sfloat),
            Format::Rgba8Unorm => Ok(TextureFormat::Rgba8Unorm),
            Format::Rgba8Snorm => Ok(TextureFormat::Rgba8Snorm),
            Format::Rgba8Srgb => Ok(TextureFormat::Rgba8Srgb),
            Format::Rgba8Uint => Ok(TextureFormat::Rgba8Uint),
            Format::Rgba8Sint => Ok(TextureFormat::Rgba8Sint),
            Format::Bgra8Unorm => Ok(TextureFormat::Bgra8Unorm),
            Format::Bgra8Srgb => Ok(TextureFormat::Bgra8Srgb),
            Format::Rg32Uint => Ok(TextureFormat::Rg32Uint),
            Format::Rg32Sint => Ok(TextureFormat::Rg32Sint),
            Format::Rg32Sfloat => Ok(TextureFormat::Rg32Sfloat),
            Format::Rgba16Uint => Ok(TextureFormat::Rgba16Uint),
            Format::Rgba16Sint => Ok(TextureFormat::Rgba16Sint),
            Format::Rgba16Sfloat => Ok(TextureFormat::Rgba16Sfloat),
            Format::Rgba32Uint => Ok(TextureFormat::Rgba32Uint),
            Format::Rgba32Sint => Ok(TextureFormat::Rgba32Sint),
            Format::Rgba32Sfloat => Ok(TextureFormat::Rgba32Sfloat),
            Format::D32Sfloat => Ok(TextureFormat::Depth32Sfloat),
            Format::D24UnormS8Uint => Ok(TextureFormat::Depth24PlusStencil8),
            _ => Err(format!("Unsupported format: {:#?}", self)),
        }
    }
}
