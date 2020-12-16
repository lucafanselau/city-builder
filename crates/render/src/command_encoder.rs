use crate::resource::buffer::BufferRange;
use crate::resource::frame::Clear;
use crate::resource::pipeline::{Rect, Viewport};
use crate::{context::GpuContext, resource::glue::Glue};
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
    fn bind_vertex_buffer(&mut self, binding: u32, buffer: &C::BufferHandle, range: BufferRange);

    fn snort_glue(&mut self, set_idx: usize, pipeline: &C::PipelineHandle, glue: &Glue<C>);

    fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>);
}
