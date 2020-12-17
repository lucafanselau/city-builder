use bytemuck::Pod;

use crate::resource::{
    glue::Glue,
    pipeline::{GraphicsPipeline, GraphicsPipelineDescriptor, RenderContext},
};
use crate::{
    command_encoder::CommandEncoder,
    resource::{
        buffer::{Buffer, BufferCopy, BufferDescriptor, BufferUsage, MemoryType},
        glue::{DescriptorSet, GlueBottle},
    },
};
use crate::{
    context::{CurrentContext, GpuContext},
    resource::glue::{Mixture, MixturePart},
};
use std::{borrow::Cow, mem::ManuallyDrop, sync::Arc};

#[derive(Debug)]
pub struct GpuResources<Context: GpuContext> {
    ctx: Arc<Context>,
}

impl<Context: GpuContext> GpuResources<Context> {
    pub fn new(ctx: Arc<Context>) -> Self {
        Self { ctx }
    }

    pub fn create_empty_buffer(&self, desc: BufferDescriptor) -> Buffer<Context> {
        let handle = self.ctx.create_buffer(&desc);
        Buffer::new(desc.name, handle, self.ctx.clone())
    }

    pub fn create_vertex_buffer<T: Pod>(
        &self,
        name: Cow<'static, str>,
        data: &T,
    ) -> Buffer<Context> {
        let size = std::mem::size_of::<T>() as u64;

        let staging_buffer = {
            let staging_desc = BufferDescriptor {
                name: format!("{}-staging", name).into(),
                size,
                memory_type: MemoryType::HostVisible,
                usage: BufferUsage::Staging,
            };

            let handle = self.ctx.create_buffer(&staging_desc);

            unsafe {
                self.ctx.write_to_buffer(&handle, data);
            }

            handle
        };

        let handle = {
            let desc = BufferDescriptor {
                name: name.clone(),
                size,
                memory_type: MemoryType::DeviceLocal,
                usage: BufferUsage::Vertex,
            };

            self.ctx.create_buffer(&desc)
        };

        self.ctx.single_shot_command(true, |cmd| {
            let copy = BufferCopy {
                src_offset: 0,
                dst_offset: 0,
                size,
            };
            cmd.copy_buffer(&staging_buffer, &handle, vec![copy]);
        });

        self.ctx.drop_buffer(staging_buffer);
        Buffer::new(name, handle, self.ctx.clone())
    }

    pub fn create_graphics_pipeline(
        &self,
        desc: GraphicsPipelineDescriptor<Context>,
        render_context: RenderContext<Context>,
    ) -> GraphicsPipeline<Context> {
        let name = desc.name.clone();
        let handle = self.ctx.create_graphics_pipeline(desc, render_context);
        GraphicsPipeline::new(name, handle, self.ctx.clone())
    }

    pub fn stir(&self, parts: Vec<MixturePart>) -> Mixture<Context> {
        let handle = self.ctx.create_descriptor_layout(parts.clone());
        Mixture::new(parts, self.ctx.clone(), handle)
    }

    pub fn bottle(&self, mixture: &Mixture<Context>) -> GlueBottle<Context> {
        let raw_handle = self.ctx.create_descriptor_set(mixture);
        let set = DescriptorSet {
            ctx: self.ctx.clone(),
            handle: ManuallyDrop::new(raw_handle),
        };
        GlueBottle::<Context>::new(set, mixture.parts.clone())
    }

    pub fn disolve(&self, glue: Glue<Context>) -> GlueBottle<Context> {
        GlueBottle::<Context>::new(glue.handle, glue.parts)
    }
}
