use crate::command_encoder::CommandEncoder;
use crate::resource::frame::{Clear, Extent3D};
use crate::resource::pipeline::{
    AttributeDescriptor, CullFace, Culling, GraphicsPipelineDescriptor, PipelineShaders,
    PipelineState, PipelineStates, PolygonMode, Primitive, Rasterizer, Rect, RenderContext,
    ShaderSource, VertexAttributeFormat, VertexBufferDescriptor, VertexInputRate, Viewport,
    Winding,
};
use crate::resource::render_pass::{
    Attachment, AttachmentLoadOp, AttachmentStoreOp, RenderPassDescriptor, SubpassDescriptor,
};
use crate::resource::{buffer::BufferRange, swap_buffer::SwapBuffer};
use crate::resources::GpuResources;
use crate::util::format::TextureLayout;
use crate::{
    context::{create_render_context, GpuContext},
    resource::glue::PartIndex,
};
use bytemuck::{Pod, Zeroable};
use log::info;
use log::warn;
use raw_window_handle::HasRawWindowHandle;
use std::borrow::Borrow;
use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

#[derive(Copy, Clone, Zeroable, Pod)]
#[repr(C)]
struct Vertex {
    pos: [f32; 4],
}

#[derive(Copy, Clone, Zeroable, Pod)]
#[repr(C)]
struct Offset {
    offset: [f32; 4],
}

#[derive(Copy, Clone)]
#[repr(C)]
struct ShaderCamera {
    view_projection: [[f32; 4]; 4],
}

pub fn test_renderer<W: HasRawWindowHandle>(w: &W, extent: (u32, u32)) {
    let ctx = Arc::new(create_render_context(w));

    info!("Surface format is {:#?}", ctx.get_surface_format());

    let resources = GpuResources::new(ctx.clone());

    let mut running = true;
    let startup = Instant::now();
    let mut last_frame = startup.clone();
    let mut counter = 0f64;
    let mut frames = 0u32;
    warn!("starting..");
    while running {
        let elapsed = startup.elapsed().as_secs_f32();
        if elapsed > 6.0 {
            running = false;
        }

        let offset = Offset {
            offset: [elapsed.sin(), elapsed.cos(), elapsed.tan(), 0.0],
        };

        offset_buffer.write(offset);

        let now = Instant::now();

        {
            frames += 1;
            let delta = now.duration_since(last_frame).as_secs_f64();
            // warn!("frame: delta: {}", delta);
            last_frame = now.clone();
            counter += delta;
            if counter >= 1f64 {
                counter -= 1f64;
                info!("fps: {}", frames);
                frames = 0;
            }
        }

        let (index, swapchain_image) = ctx.new_frame();

        offset_buffer.frame(index);

        let framebuffer = ctx.create_framebuffer(
            &render_pass,
            vec![swapchain_image.borrow()],
            Extent3D {
                width: extent.0,
                height: extent.1,
                depth: 1,
            },
        );

        let viewport = Viewport {
            rect: Rect {
                x: 0,
                y: 0,
                width: extent.0 as i16,
                height: extent.1 as i16,
            },
            depth: 0.0..1.0,
        };

        let frame_commands = ctx.render_command(|cmd| {
            cmd.begin_render_pass(
                &render_pass,
                &framebuffer,
                viewport.clone().rect,
                vec![Clear::Color(0.34, 0.12, 0.12, 1.0)],
            );

            cmd.bind_graphics_pipeline(&pipeline);

            cmd.set_viewport(0, viewport.clone());
            cmd.set_scissor(0, viewport.rect);

            cmd.bind_vertex_buffer(0, vertex_buffer.deref(), BufferRange::WHOLE);

            cmd.snort_glue(0, &pipeline, &glue_drops[index as usize]);

            cmd.draw(0..3, 0..1);

            cmd.end_render_pass();
        });

        ctx.end_frame(swapchain_image, frame_commands);

        ctx.drop_framebuffer(framebuffer);
    }

    ctx.wait_idle();

    ctx.drop_render_pass(render_pass);
}
