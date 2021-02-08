use crate::resource::frame::Clear;
use crate::resource::pipeline::{Rect, Viewport};
use crate::{context::GpuContext, resource::glue::Glue};
use crate::{
    prelude::ShaderType,
    resource::buffer::{BufferCopy, BufferRange},
};
use std::ops::Range;

pub trait CommandEncoder<C: GpuContext + ?Sized> {
    fn begin_render_pass<I: IntoIterator<Item = Clear>>(
        &mut self,
        render_pass: &C::RenderPassHandle,
        frame_buffer: &C::Framebuffer,
        render_area: Rect,
        clear_values: I,
    );

    fn end_render_pass(&mut self);

    fn set_viewport(&mut self, index: u32, viewport: Viewport);
    fn set_scissor(&mut self, index: u32, scissor: Rect);

    fn bind_graphics_pipeline(&mut self, pipeline: &C::PipelineHandle);

    fn push_constants(
        &mut self,
        pipeline: &C::PipelineHandle,
        shader: ShaderType,
        offset: u32,
        data: &[u32],
    );

    fn bind_vertex_buffer(&mut self, binding: u32, buffer: &C::BufferHandle, range: BufferRange);

    fn snort_glue(&mut self, set_idx: usize, pipeline: &C::PipelineHandle, glue: &Glue<C>);

    fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>);

    fn copy_buffer<I>(&mut self, src: &C::BufferHandle, dst: &C::BufferHandle, regions: I)
    where
        I: IntoIterator<Item = BufferCopy>;
}
