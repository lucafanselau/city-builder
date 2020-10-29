mod gfx_context;
mod types;

use raw_window_handle::HasRawWindowHandle;
use types::*;

use gfx_backend_vulkan as graphics_backend;

pub trait RenderContext {
    fn create_initialized_buffer(&self) -> BufferHandle;
}

pub fn create_render_context<W: HasRawWindowHandle>(window: &W) -> impl RenderContext {
    gfx_context::GfxRenderContext::<graphics_backend::Backend>::new(window)
}
