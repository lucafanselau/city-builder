use crate::context::{CurrentContext, GpuContext};
use crate::resource::buffer::{Buffer, BufferDescriptor};
use crate::resource::pipeline::{GraphicsPipeline, GraphicsPipelineDescriptor, RenderContext};
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

    pub fn create_graphics_pipeline(
        &self,
        desc: &GraphicsPipelineDescriptor,
        render_context: RenderContext<CurrentContext>,
    ) -> GraphicsPipeline {
        let handle = self.ctx.create_graphics_pipeline(desc, render_context);
        GraphicsPipeline::new(desc.name.clone(), handle, self.ctx.clone())
    }
}
