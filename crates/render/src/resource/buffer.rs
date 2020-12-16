use crate::context::{CurrentContext, GpuContext};
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
}

#[derive(Clone, Debug)]
pub struct BufferDescriptor {
    pub name: Cow<'static, str>,
    pub size: u64,
    pub memory_type: MemoryType,
    pub usage: BufferUsage,
}

type BufferHandle = <CurrentContext as GpuContext>::BufferHandle;

#[derive(Debug)]
pub struct Buffer {
    name: Cow<'static, str>,
    ctx: Arc<CurrentContext>,
    handle: ManuallyDrop<BufferHandle>,
}

impl Buffer {
    pub fn new(name: Cow<'static, str>, handle: BufferHandle, ctx: Arc<CurrentContext>) -> Self {
        Self {
            name,
            ctx,
            handle: ManuallyDrop::new(handle),
        }
    }

    pub fn get_handle(&self) -> &BufferHandle {
        self.handle.deref()
    }
}

impl Deref for Buffer {
    type Target = BufferHandle;

    fn deref(&self) -> &Self::Target {
        self.handle.deref()
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            self.ctx.drop_buffer(ManuallyDrop::take(&mut self.handle));
        }
    }
}
