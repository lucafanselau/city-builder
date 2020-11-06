use crate::resource::buffer::BufferDescriptor;
use bytemuck::Pod;
use raw_window_handle::HasRawWindowHandle;
use std::fmt::Debug;

pub trait GpuContext: Send + Sync {
    type BufferHandle: Send + Sync + Debug;
    type PipelineHandle: Send + Sync + Debug;

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

    // Pipelines

    fn create_pipeline(&self, desc: &PipelineDescriptor) -> Self::PipelineHandle;
    fn drop_pipeline(&self, pipeline: Self::PipelineHandle);

    // fn create_texture()
    // fn create_initialized_texture()
    // And maybe something more sophisticated for attachments

    fn create_command_encoder(&self) {}

    fn submit_command(&self) {}

    fn create_shader_module(&self) {}

    fn configure_swap_chain(&self) {}
}

// here would be like #[cfg(feature = "gfx")] or something if we make this plug and play
use gfx_backend_vulkan as graphics_backend;
pub type CurrentContext = crate::gfx::gfx_context::GfxContext<graphics_backend::Backend>;

pub fn create_render_context<W: HasRawWindowHandle>(window: &W) -> CurrentContext {
    CurrentContext::new(window)
}
