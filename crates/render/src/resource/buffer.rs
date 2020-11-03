use crate::context::GpuContext;
use std::borrow::Cow;
use std::mem::ManuallyDrop;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MemoryType {
    DeviceLocal,
    HostVisible,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BufferUsage {
    Uniform,
    Vertex,
    Index,
}

#[derive(Clone, Debug)]
pub struct BufferDescriptor {
    pub name: Cow<'static, str>,
    pub size: u64,
    pub memory_type: MemoryType,
    pub usage: BufferUsage,
}

#[derive(Debug)]
pub struct Buffer<C: GpuContext> {
    name: Cow<'static, str>,
    ctx: Arc<C>,
    handle: ManuallyDrop<<C as GpuContext>::BufferHandle>,
}

impl<C: GpuContext> Buffer<C> {
    pub fn new(
        name: Cow<'static, str>,
        handle: <C as GpuContext>::BufferHandle,
        ctx: Arc<C>,
    ) -> Self {
        Self {
            name,
            ctx,
            handle: ManuallyDrop::new(handle),
        }
    }
}

impl<C: GpuContext> Drop for Buffer<C> {
    fn drop(&mut self) {
        unsafe {
            self.ctx.drop_buffer(ManuallyDrop::take(&mut self.handle));
        }
    }
}
