use crate::resource::buffer::BufferDescriptor;
use crate::resource::frame::Extent3D;
use crate::resource::pipeline::{GraphicsPipelineDescriptor, RenderContext, ShaderSource};
use crate::util::format::TextureFormat;
use bytemuck::Pod;
use gfx_backend_vulkan as graphics_backend;
use raw_window_handle::HasRawWindowHandle;
use std::borrow::Borrow;
use std::fmt::Debug;

pub trait GpuContext: Send + Sync {
    type BufferHandle: Send + Sync + Debug;
    type PipelineHandle: Send + Sync + Debug;
    type RenderPassHandle: Send + Sync + Debug;
    type ShaderCode: Debug + Send + Sized;
    type ImageView: Debug + Send + Sync;
    type Framebuffer: Debug + Send + Sync;
    type CommandBuffer: Debug + Send + Sync;
    /// eg. The Command buffer in recording state
    type CommandEncoder: CommandEncoder<Self> + Debug + Send + Sync;
    type SwapchainImage: Borrow<Self::ImageView> + Debug + Send + Sync;

    // Oke we will need to create abstractions for all of these first
    // fn create_initialized_buffer(
    //     &self,
    //     _data: &[u8], /*, probably like a memory type*/
    // ) -> Self::BufferHandle

    // Buffer Sh**

    /// Create a buffer that is not bound to any memory, see bind_memory for that
    fn create_buffer(&self, desc: &BufferDescriptor) -> Self::BufferHandle;

    /// Safety: this is only valid for buffers that are writable, eg. memory_type == HostVisible
    unsafe fn write_to_buffer<D: Pod>(&self, buffer: &Self::BufferHandle, data: &D);

    /// Drop a Buffer handle
    fn drop_buffer(&self, buffer: Self::BufferHandle);

    // Render Passes
    fn create_render_pass(&self, desc: &RenderPassDescriptor) -> Self::RenderPassHandle;
    fn drop_render_pass(&self, rp: Self::RenderPassHandle);

    // Pipelines
    fn create_graphics_pipeline(
        &self,
        desc: &GraphicsPipelineDescriptor,
        render_context: RenderContext<Self>,
    ) -> Self::PipelineHandle;
    fn drop_pipeline(&self, pipeline: Self::PipelineHandle);

    // NOTE(luca): Maybe this should not be provided by context, or like more sleek, but for now
    //  this is enough
    fn compile_shader(&self, source: ShaderSource) -> Self::ShaderCode;

    // TODO: if we want to support multi surface or headless drawing a surface can not be bound to
    //  the context...
    /// Will return the format of the created surface
    fn get_surface_format(&self) -> TextureFormat;

    // Framebuffers
    fn create_framebuffer<I>(
        &self,
        rp: &Self::RenderPassHandle,
        attachments: I,
        extent: Extent3D,
    ) -> Self::Framebuffer
    where
        I: IntoIterator,
        I::Item: Borrow<Self::ImageView>;

    fn drop_framebuffer(&self, fb: Self::Framebuffer);

    // Rendering API
    //
    // This API is temporary and im not quite sure how to abstract that away
    fn new_frame(&self) -> Self::SwapchainImage;
    fn end_frame(&self, swapchain_image: Self::SwapchainImage, frame_commands: Self::CommandBuffer);

    fn render_command(&self, cb: impl FnOnce(&mut Self::CommandEncoder)) -> Self::CommandBuffer;

    fn wait_idle(&self);
}

// here would be like #[cfg(feature = "gfx")] or something if we make this plug and play
use crate::command_encoder::CommandEncoder;
use crate::resource::render_pass::RenderPassDescriptor;

pub type CurrentContext = crate::gfx::gfx_context::GfxContext<graphics_backend::Backend>;

pub fn create_render_context<W: HasRawWindowHandle>(window: &W) -> CurrentContext {
    CurrentContext::new(window)
}
