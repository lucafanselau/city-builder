use crate::resource::{
    glue::Mixture,
    pipeline::{GraphicsPipelineDescriptor, RenderContext, ShaderSource},
};
use crate::util::format::TextureFormat;
use crate::{
    command_encoder::CommandEncoder,
    resource::{buffer::BufferDescriptor, glue::MixturePart},
};
use crate::{
    graph::Graph,
    resource::{frame::Extent3D, glue::DescriptorWrite, render_pass::RenderPassDescriptor},
};
use bytemuck::Pod;
use raw_window_handle::HasRawWindowHandle;
use std::borrow::Borrow;
use std::fmt::Debug;

pub trait GpuBuilder {
    type Context: GpuContext;

    fn new() -> Self;

    fn create_surface<W: HasRawWindowHandle>(
        &mut self,
        window: &W,
    ) -> <Self::Context as GpuContext>::SurfaceHandle;

    fn build(self) -> Self::Context;
}

pub trait GpuContext: Send + Sync {
    type SurfaceHandle: Send + Sync + Clone; // TODO: Maybe add specific Trait
    type BufferHandle: Send + Sync + Debug;
    type PipelineHandle: Send + Sync + Debug;
    type RenderPassHandle: Send + Sync + Debug;
    type ShaderCode: Debug + Send + Sized;
    type ImageView: Debug + Send + Sync;
    type Framebuffer: Debug + Send + Sync;
    type CommandBuffer: Debug + Send + Sync;
    type DescriptorLayout: Debug + Send + Sync;
    type DescriptorSet: Debug + Send + Sync;
    /// eg. The Command buffer in recording state
    type CommandEncoder: CommandEncoder<Self> + Debug + Send + Sync;
    type SwapchainImage: Borrow<Self::ImageView> + Debug + Send + Sync;
    type ContextGraph: Graph;
    // Oke we will need to create abstractions for all of these first
    // fn create_initialized_buffer(
    //     &self,
    //     _data: &[u8], /*, probably like a memory type*/
    // ) -> Self::BufferHandle

    // Buffer Sh**

    /// Create a buffer that is not bound to any memory, see bind_memory for that
    fn create_buffer(&self, desc: &BufferDescriptor) -> Self::BufferHandle;

    /// # Safety
    ///
    /// this is only valid for buffers that are writable, eg. memory_type == HostVisible
    unsafe fn write_to_buffer<D: Pod>(&self, buffer: &Self::BufferHandle, data: &D);

    /// Drop a Buffer handle
    fn drop_buffer(&self, buffer: Self::BufferHandle);

    // Render Passes
    fn create_render_pass(&self, desc: &RenderPassDescriptor) -> Self::RenderPassHandle;
    fn drop_render_pass(&self, rp: Self::RenderPassHandle);

    // Pipelines
    fn create_graphics_pipeline(
        &self,
        desc: GraphicsPipelineDescriptor<Self>,
        render_context: RenderContext<Self>,
    ) -> Self::PipelineHandle
    where
        Self: Sized;
    fn drop_pipeline(&self, pipeline: Self::PipelineHandle);

    // NOTE(luca): Maybe this should not be provided by context, or like more sleek, but for now
    //  this is enough
    fn compile_shader(&self, source: ShaderSource) -> Self::ShaderCode;

    // TODO: if we want to support multi surface or headless drawing a surface can not be bound to
    //  the context...
    /// Will return the format of the created surface
    // fn get_surface_format(&self) -> TextureFormat;

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

    // Descriptors from here on
    fn create_descriptor_layout<I>(&self, parts: I) -> Self::DescriptorLayout
    where
        I: IntoIterator<Item = MixturePart>;

    fn drop_descriptor_layout(&self, handle: Self::DescriptorLayout);

    fn create_descriptor_set(&self, layout: &Mixture<Self>) -> Self::DescriptorSet;
    fn drop_descriptor_set(&self, handle: Self::DescriptorSet);

    fn update_descriptor_set(
        &self,
        handle: &Self::DescriptorSet,
        writes: Vec<DescriptorWrite<Self>>,
    );

    // Single Shot Commands -> Transfer etc.
    // fn single_shot_command(&self, should_wait: bool, cb: impl FnOnce(&mut Self::CommandEncoder));

    // DEPRECATED: In favor of the new graph api
    // Rendering API
    //
    // This API is temporary and im not quite sure how to abstract that away
    // fn new_frame(&self) -> (u32, Self::SwapchainImage);
    // fn end_frame(&self, swapchain_image: Self::SwapchainImage, frame_commands: Self::CommandBuffer);
    // fn render_command(&self, cb: impl FnOnce(&mut Self::CommandEncoder)) -> Self::CommandBuffer;
    // fn swapchain_image_count(&self) -> usize;
    // fn handle_resize(&self, size: Extent2D);

    fn wait_idle(&self);

    // Create a Graph object (will replace the Rendering API above)
    fn create_graph(&self, surface: Self::SurfaceHandle) -> Self::ContextGraph;
}
