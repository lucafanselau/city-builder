use crate::context::{CurrentContext, GpuContext};
use crate::resource::buffer::{Buffer, BufferDescriptor};
use crate::resource::pipeline::{GraphicsPipeline, GraphicsPipelineDescriptor};
use std::sync::Arc;

#[derive(Debug)]
pub struct GpuResources {
    ctx: Arc<CurrentContext>,
}

impl GpuResources {
    pub fn new(ctx: Arc<CurrentContext>) -> Self {
        Self { ctx }
    }

    pub fn create_empty_buffer(&self, desc: BufferDescriptor) -> Buffer {
        let handle = self.ctx.create_buffer(&desc);
        Buffer::new(desc.name, handle, self.ctx.clone())
    }

    pub fn create_graphics_pipeline(&self, desc: &GraphicsPipelineDescriptor) -> GraphicsPipeline {
        let handle = self.ctx.create_graphics_pipeline(desc);
        GraphicsPipeline::new(desc.name.clone(), handle, self.ctx.clone())
    }
}
