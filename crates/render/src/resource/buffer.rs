use crate::context::GpuContext;
use gfx_hal::buffer::Usage;
use std::borrow::Cow;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct BufferDescriptor {
    pub(crate) name: Cow<'static, str>,
    pub(crate) size: u64,
    // TODO: This is not acceptable, because it is outside of the gfx folder
    pub(crate) usage: Usage,
}

#[derive(Debug)]
pub struct Buffer<C: GpuContext> {
    ctx: Arc<C>,
    handle: <C as GpuContext>::BufferHandle,
}
