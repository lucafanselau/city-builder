use crate::context::GpuContext;
use crate::resource::buffer::{Buffer, BufferDescriptor};
use std::sync::Arc;

#[derive(Debug)]
pub struct GpuResources<C: GpuContext> {
    ctx: Arc<C>,
}

impl<C: GpuContext> GpuResources<C> {
    pub fn new(ctx: Arc<C>) -> Self {
        Self { ctx }
    }

    pub fn create_empty_buffer(&self, desc: BufferDescriptor) -> Buffer<C> {
        let handle = self.ctx.create_buffer(&desc);
        Buffer::<C>::new(desc.name, handle, self.ctx.clone())
    }
}
