use crate::command_encoder::CommandEncoder;
use crate::context::GpuContext;
use crate::gfx::compat::ToHalType;
use crate::gfx::gfx_context::GfxContext;
use crate::resource::frame::Clear;
use crate::resource::pipeline::{Rect, Viewport};
use bytemuck::__core::ops::Range;
use gfx_hal::adapter::Adapter;
use gfx_hal::command::{ClearValue, CommandBuffer, SubpassContents};
use gfx_hal::Backend;
use std::sync::Arc;

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
            self.command.bind_graphics_pipeline(pipeline);
        }
    }

    fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>) {
        unsafe {
            self.command.draw(vertices, instances);
        }
    }
}
