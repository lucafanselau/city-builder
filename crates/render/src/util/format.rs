// This is like a straight copy from wgpu (if we need more we can add those later)
#[derive(Debug, Clone, Copy)]
pub enum TextureFormat {
    R8Unorm,
    R8Snorm,
    R8Uint,
    R8Sint,
    R16Uint,
    R16Sint,
    R16Sfloat,
    Rg8Unorm,
    Rg8Snorm,
    Rg8Uint,
    Rg8Sint,
    R32Uint,
    R32Sint,
    R32Sfloat,
    Rg16Uint,
    Rg16Sint,
    Rg16Sfloat,
    Rgba8Unorm,
    Rgba8Srgb,
    Rgba8Snorm,
    Rgba8Uint,
    Rgba8Sint,
    Bgra8Unorm,
    Bgra8Srgb,
    Rg32Uint,
    Rg32Sint,
    Rg32Sfloat,
    Rgba16Uint,
    Rgba16Sint,
    Rgba16Sfloat,
    Rgba32Uint,
    Rgba32Sint,
    Rgba32Sfloat,
    Depth32Sfloat,
    Depth24PlusStencil8,
}

/// This is basically a copy from gfx-hal
#[derive(Debug, Clone)]
pub enum TextureLayout {
    /// General purpose, no restrictions on usage.
    General,
    /// Must only be used as a color attachment in a framebuffer.
    ColorAttachmentOptimal,
    /// Must only be used as a depth attachment in a framebuffer.
    DepthStencilAttachmentOptimal,
    /// Must only be used as a depth attachment in a framebuffer,
    /// or as a read-only depth or stencil buffer in a shader.
    DepthStencilReadOnlyOptimal,
    /// Must only be used as a read-only image in a shader.
    ShaderReadOnlyOptimal,
    /// Must only be used as the source for a transfer command.
    TransferSrcOptimal,
    /// Must only be used as the destination for a transfer command.
    TransferDstOptimal,
    /// No layout, does not support device access.  Only valid as a
    /// source layout when transforming data to a specific destination
    /// layout or initializing data.  Does NOT guarentee that the contents
    /// of the source buffer are preserved.
    Undefined,
    /// Like `Undefined`, but does guarentee that the contents of the source
    /// buffer are preserved.
    Preinitialized,
    /// The layout that an image must be in to be presented to the display.
    Present,
}

#[derive(Debug, Clone)]
pub enum ImageAccess {
    InputAttachmentRead,
    ShaderRead,
    ShaderWrite,
    ColorAttachmentRead,
    ColorAttachmentWrite,
    DepthStencilAttachmentRead,
    DepthStencilAttachmentWrite,
    TransferRead,
    TransferWrite,
    HostRead,
    HostWrite,
    MemoryRead,
    MemoryWrite,
}

#[derive(Debug, Clone, Copy)]
pub enum ImageTiling {
    Linear,
    Optimal,
}
