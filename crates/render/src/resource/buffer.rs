use crate::context::GpuContext;
use std::borrow::Cow;
use std::mem::ManuallyDrop;
use std::ops::{Deref, Range};
use std::sync::Arc;

/// Used for binding the buffer in a command encoder
#[derive(Debug, Clone)]
pub struct BufferRange {
    pub offset: u64,
    /// When None the whole buffer is used
    pub size: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct BufferCopy {
    pub src_offset: u64,
    pub dst_offset: u64,
    pub size: u64,
}

impl BufferRange {
    pub const WHOLE: Self = Self {
        offset: 0,
        size: None,
    };
}

impl From<Range<u64>> for BufferRange {
    fn from(r: Range<u64>) -> Self {
        Self {
            offset: r.start,
            size: Some(r.end - r.start),
        }
    }
}

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
    Staging,
}

#[derive(Clone, Debug)]
pub struct BufferDescriptor<'a> {
    pub name: Cow<'a, str>,
    pub size: u64,
    pub memory_type: MemoryType,
    pub usage: BufferUsage,
}

#[derive(Debug)]
pub struct Buffer<Context: GpuContext> {
    name: String,
    ctx: Arc<Context>,
    handle: ManuallyDrop<Context::BufferHandle>,
}

impl<Context: GpuContext> Buffer<Context> {
    pub fn new(name: String, handle: Context::BufferHandle, ctx: Arc<Context>) -> Self {
        Self {
            name,
            ctx,
            handle: ManuallyDrop::new(handle),
        }
    }

    pub fn get_handle(&self) -> &Context::BufferHandle {
        self.handle.deref()
    }
}

impl<Context: GpuContext> Deref for Buffer<Context> {
    type Target = Context::BufferHandle;

    fn deref(&self) -> &Self::Target {
        self.handle.deref()
    }
}

impl<Context: GpuContext> Drop for Buffer<Context> {
    fn drop(&mut self) {
        unsafe {
            self.ctx.drop_buffer(ManuallyDrop::take(&mut self.handle));
        }
    }
}
