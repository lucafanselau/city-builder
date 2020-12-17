use std::{borrow::Cow, sync::atomic::Ordering};
use std::{
    ops::Deref,
    sync::{atomic::AtomicBool, Arc},
};

use bytemuck::Pod;
use parking_lot::Mutex;

use crate::context::GpuContext;

use super::buffer::{BufferDescriptor, BufferUsage, MemoryType};

#[derive(Debug)]
struct FrameBuffer<Context: GpuContext> {
    buffer: <Context as GpuContext>::BufferHandle,
    should_update: AtomicBool,
}

#[derive(Debug)]
pub struct SwapBuffer<Context: GpuContext, T>
where
    T: Sized + Pod + Send,
{
    pub(crate) name: Cow<'static, str>,
    ctx: Arc<Context>,
    frames: Vec<FrameBuffer<Context>>,
    current_data: Mutex<T>,
}

impl<Context: GpuContext, T> SwapBuffer<Context, T>
where
    T: Sized + Pod + Send,
{
    pub fn new(
        ctx: Arc<Context>,
        name: Cow<'static, str>,
        frames_in_flight: usize,
        initial_data: T,
    ) -> Self {
        let mut frames = Vec::with_capacity(frames_in_flight);

        for i in 0..frames_in_flight {
            let desc = BufferDescriptor {
                name: format!("SwapBuffer-{}-frame-{}", name, i).into(),
                size: std::mem::size_of::<T>() as u64,
                memory_type: MemoryType::HostVisible,
                usage: BufferUsage::Uniform,
            };

            let handle = ctx.create_buffer(&desc);
            unsafe {
                ctx.write_to_buffer(&handle, &initial_data);
            };
            frames.push(FrameBuffer {
                buffer: handle,
                should_update: AtomicBool::new(false),
            });
        }

        Self {
            name,
            ctx,
            frames,
            current_data: Mutex::new(initial_data),
        }
    }

    pub fn write(&self, new_data: T) {
        {
            let mut lock = self.current_data.lock();
            *lock = new_data;
        }
        // And tell every frame that it should update
        self.frames
            .iter()
            .for_each(|f| f.should_update.store(true, Ordering::Release));
    }

    pub fn get(&self, frame_idx: u32) -> &<Context as GpuContext>::BufferHandle {
        self.frame(frame_idx);
        &self.frames.get(frame_idx as usize).unwrap().buffer
    }

    /// When you are not get(ting) a buffer every frame, you will need to call this function, to update the buffer
    pub fn frame(&self, frame_idx: u32) {
        match self.frames.get(frame_idx as usize) {
            Some(frame) => {
                if frame.should_update.load(Ordering::Relaxed) {
                    // if we need to update now would be the time
                    let data = self.current_data.lock();
                    unsafe {
                        self.ctx.write_to_buffer(&frame.buffer, data.deref());
                    };
                }
            }
            None => panic!("[SwapBuffer] (frame) index out of bounds: {}", frame_idx),
        }
    }
}
