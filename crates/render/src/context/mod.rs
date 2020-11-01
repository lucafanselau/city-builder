use raw_window_handle::HasRawWindowHandle;
use std::fmt::Debug;

use crate::gfx::gfx_context::GfxContext;
use crate::resource::buffer::BufferDescriptor;
use gfx_backend_vulkan as graphics_backend;

pub trait GpuContext: Send + Sync {
    type BufferHandle: Send + Sync + Debug;

    // Oke we will need to create abstractions for all of these first
    // fn create_initialized_buffer(
    //     &self,
    //     _data: &[u8], /*, probably like a memory type*/
    // ) -> Self::BufferHandle

    /// Create a buffer that is not bound to any memory, see bind_memory for that
    fn create_buffer(&self, desc: BufferDescriptor) -> Self::BufferHandle;

    /// Drop a Buffer handle
    fn drop_buffer(&self, buffer: Self::BufferHandle);

    // fn create_texture()
    // fn create_initialized_texture()
    // And maybe something more sophisticated for attachments

    fn create_command_encoder(&self) {}

    fn submit_command(&self) {}

    fn create_shader_module(&self) {}
    fn create_pipeline(&self) {}

    fn configure_swap_chain(&self) {}
}

pub fn create_render_context<W: HasRawWindowHandle>(window: &W) -> impl GpuContext {
    GfxContext::<graphics_backend::Backend>::new(window)
}
