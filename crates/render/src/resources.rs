use crate::resource::pipeline::{GraphicsPipeline, GraphicsPipelineDescriptor, RenderContext};
use crate::resource::{
    buffer::{Buffer, BufferDescriptor},
    glue::GlueBottle,
};
use crate::{
    context::{CurrentContext, GpuContext},
    resource::glue::{Mixture, MixturePart},
};
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
        desc: GraphicsPipelineDescriptor<CurrentContext>,
        render_context: RenderContext<CurrentContext>,
    ) -> GraphicsPipeline {
        let name = desc.name.clone();
        let handle = self.ctx.create_graphics_pipeline(desc, render_context);
        GraphicsPipeline::new(name, handle, self.ctx.clone())
    }

    pub fn stir(&self, parts: Vec<MixturePart>) -> Mixture<CurrentContext> {
        let handle = self.ctx.create_descriptor_layout(parts.clone());
        Mixture::new(parts, self.ctx.clone(), handle)
    }

    pub fn bottle(&self, mixture: &Mixture<CurrentContext>) -> GlueBottle<CurrentContext> {
        let set = self.ctx.create_descriptor_set(mixture);
        GlueBottle::<CurrentContext>::new(self.ctx.clone(), set, mixture.parts.clone())
    }
}
