use bytemuck::__core::ops::Range;
use render::context::GpuContext;
use render::resource::buffer::BufferRange;
use render::resource::frame::Clear;
use render::resource::pipeline::{Rect, Viewport};
use render::{command_encoder::CommandEncoder, resource::glue::Glue};

use gfx_hal::command::{ClearValue, CommandBuffer, SubpassContents};
use gfx_hal::Backend;

use std::iter;

use crate::{compat::ToHalType, context::GfxContext};

#[derive(Debug)]
pub struct GfxCommand<B: Backend> {
    command: B::CommandBuffer,
}

impl<B: Backend> GfxCommand<B> {
    pub(crate) fn new(command: B::CommandBuffer) -> Self {
        Self { command }
    }

    pub(crate) fn into_inner(self) -> B::CommandBuffer {
        self.command
    }
}

impl<B: Backend> CommandEncoder<GfxContext<B>> for GfxCommand<B> {
    fn begin_render_pass<I: IntoIterator<Item = Clear>>(
        &mut self,
        render_pass: &<GfxContext<B> as GpuContext>::RenderPassHandle,
        frame_buffer: &<GfxContext<B> as GpuContext>::Framebuffer,
        render_area: Rect,
        clear_values: I,
    ) {
        let clear_values: Vec<ClearValue> =
            clear_values.into_iter().map(|cv| cv.convert()).collect();

        unsafe {
            self.command.begin_render_pass(
                render_pass,
                frame_buffer,
                render_area.convert(),
                clear_values,
                SubpassContents::Inline,
            )
        }
    }

    fn end_render_pass(&mut self) {
        unsafe {
            self.command.end_render_pass();
        }
    }

    fn set_viewport(&mut self, index: u32, viewport: Viewport) {
        unsafe {
            let hal_viewport = viewport.convert();
            self.command.set_viewports(index, vec![&hal_viewport]);
        }
    }

    fn set_scissor(&mut self, index: u32, scissor: Rect) {
        unsafe {
            let hal_scissor = scissor.convert();
            self.command.set_scissors(index, vec![&hal_scissor]);
        }
    }

    fn bind_graphics_pipeline(&mut self, pipeline: &<GfxContext<B> as GpuContext>::PipelineHandle) {
        unsafe {
            self.command.bind_graphics_pipeline(&pipeline.0);
        }
    }

    fn push_constants(
        &mut self,
        pipeline: &<GfxContext<B> as GpuContext>::PipelineHandle,
        shader: render::prelude::ShaderType,
        offset: u32,
        data: &[u32],
    ) {
        unsafe {
            self.command
                .push_graphics_constants(&pipeline.1, shader.convert(), offset, data)
        }
    }

    fn bind_vertex_buffer(
        &mut self,
        binding: u32,
        buffer: &<GfxContext<B> as GpuContext>::BufferHandle,
        range: BufferRange,
    ) {
        unsafe {
            self.command
                .bind_vertex_buffers(binding, iter::once((&buffer.0, range.convert())))
        }
    }

    fn snort_glue(
        &mut self,
        set_idx: usize,
        pipeline: &<GfxContext<B> as GpuContext>::PipelineHandle,
        glue: &Glue<GfxContext<B>>,
    ) {
        unsafe {
            self.command.bind_graphics_descriptor_sets(
                &pipeline.1,
                set_idx,
                vec![&glue.handle.handle.1],
                Vec::<&u32>::new(),
            )
        }
    }

    fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>) {
        unsafe {
            self.command.draw(vertices, instances);
        }
    }

    fn copy_buffer<I>(
        &mut self,
        src: &<GfxContext<B> as GpuContext>::BufferHandle,
        dst: &<GfxContext<B> as GpuContext>::BufferHandle,
        regions: I,
    ) where
        I: IntoIterator<Item = render::resource::buffer::BufferCopy>,
    {
        let regions: Vec<gfx_hal::command::BufferCopy> =
            regions.into_iter().map(|r| r.convert()).collect();
        unsafe {
            self.command.copy_buffer(&src.0, &dst.0, regions);
        }
    }
}
