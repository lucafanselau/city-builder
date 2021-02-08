use crate::plumber::Plumber;
use crate::{command::GfxCommand, context_builder::GfxBuilder};
use crate::{
    compat::{HalCompatibleSubpassDescriptor, ToHalType},
    graph::GfxGraph,
};
use crate::{
    graph::builder::GfxGraphBuilder,
    heapy::{AllocationIndex, Heapy},
};
use bytemuck::Pod;
use gfx_hal::pass::{Attachment, SubpassDependency, SubpassDesc};
use gfx_hal::queue::QueueFamilyId;
use gfx_hal::window::PresentationSurface;

use gfx_hal::{device::Device, Backend};
use parking_lot::Mutex;
use render::resource::frame::{Extent2D, Extent3D};
use render::resource::glue::Mixture;
use render::resource::pipeline::{GraphicsPipelineDescriptor, RenderContext, ShaderSource};
use render::resource::render_pass::RenderPassDescriptor;

use render::{
    context::GpuBuilder,
    resource::buffer::{BufferDescriptor, BufferUsage},
};
use render::{context::GpuContext, resource::glue::DescriptorWrite};
use std::borrow::Borrow;
use std::{ops::Deref, sync::Arc};

use super::pool::{LayoutHandle, Pool, SetHandle};

#[derive(Debug)]
pub(crate) struct Queues<B: Backend> {
    pub(crate) graphics: Mutex<B::CommandQueue>,
    pub(crate) graphics_family: QueueFamilyId,
    pub(crate) compute: Mutex<B::CommandQueue>,
    pub(crate) compute_family: QueueFamilyId,
}

use gfx_backend_vulkan as graphics_backend;
pub type ContextBuilder = GfxBuilder<graphics_backend::Backend>;
pub type Context = <ContextBuilder as GpuBuilder>::Context;

/// This is the GFX-hal implementation of the Rendering Context described in mod.rs
#[derive(Debug)]
pub struct GfxContext<B: Backend>
where
    B::Device: Send + Sync,
{
    pub(crate) instance: B::Instance,
    pub(crate) device: Arc<B::Device>,
    pub(crate) adapter: Arc<gfx_hal::adapter::Adapter<B>>,
    pub(crate) queues: Arc<Queues<B>>,
    // Memory managment
    pub(crate) heapy: Heapy<B>,
    // Pipelines
    pub(crate) plumber: Plumber<B>,
    // DEPRECATED: Swapchain
    // pub(crate) swapper: Swapper<B>,nn
    // Pool -> Descriptor Sets
    pub(crate) pool: Pool<B>,
}

impl<B: Backend> GfxContext<B> {
    #[allow(dead_code)]
    pub(crate) fn get_raw(&self) -> &B::Device {
        &self.device
    }
}

impl<B: Backend> GpuContext for GfxContext<B> {
    type SurfaceHandle = (Arc<Mutex<B::Surface>>, Extent2D);
    type BufferHandle = (B::Buffer, AllocationIndex);
    type PipelineHandle = (B::GraphicsPipeline, B::PipelineLayout);
    type RenderPassHandle = B::RenderPass;
    type ShaderCode = Vec<u32>;
    type ImageView = B::ImageView;
    type Framebuffer = B::Framebuffer;
    type CommandBuffer = B::CommandBuffer;
    type DescriptorLayout = LayoutHandle<B>;
    type DescriptorSet = SetHandle<B>;
    type CommandEncoder = GfxCommand<B>;
    type SwapchainImage =
        <<B as gfx_hal::Backend>::Surface as PresentationSurface<B>>::SwapchainImage;
    type Graph = GfxGraph<B>;
    type GraphBuilder = GfxGraphBuilder<B>;

    fn create_buffer(&self, desc: &BufferDescriptor) -> Self::BufferHandle {
        unsafe {
            match self.device.create_buffer(desc.size, get_buffer_usage(desc)) {
                Ok(mut buffer) => {
                    let requirements = self.device.get_buffer_requirements(&buffer);
                    let allocation =
                        self.heapy
                            .alloc(requirements.size, desc.memory_type, Some(requirements));
                    self.heapy.bind_buffer(&allocation, &mut buffer);

                    self.device.set_buffer_name(&mut buffer, desc.name.deref());

                    (buffer, allocation)
                }
                Err(err) => panic!(
                    "[GfxContext] failed to create buffer [{}]: {:#?}",
                    desc.name, err
                ),
            }
        }
    }

    unsafe fn write_to_buffer<D: Pod>(&self, buffer: &Self::BufferHandle, data: &D) {
        self.heapy.write(&buffer.1, bytemuck::bytes_of(data));
    }

    unsafe fn write_to_buffer_raw(&self, buffer: &Self::BufferHandle, data: &[u8]) {
        self.heapy.write(&buffer.1, data);
    }

    fn drop_buffer(&self, buffer: Self::BufferHandle) {
        unsafe {
            self.device.destroy_buffer(buffer.0);
        }
        self.heapy.deallocate(buffer.1);
    }

    fn create_render_pass(&self, desc: &RenderPassDescriptor) -> Self::RenderPassHandle {
        let attachments: Vec<Attachment> = desc
            .attachments
            .iter()
            .map(|a| a.clone().convert())
            .collect();
        let compatible_subpasses: Vec<HalCompatibleSubpassDescriptor> =
            desc.subpasses.iter().map(|s| s.clone().convert()).collect();
        let dependencies: Vec<SubpassDependency> = desc
            .pass_dependencies
            .iter()
            .map(|d| d.clone().convert())
            .collect();
        unsafe {
            self.device
                .create_render_pass(
                    attachments,
                    compatible_subpasses.iter().map(|s| SubpassDesc {
                        colors: s.colors.as_slice(),
                        depth_stencil: s.depth_stencil.as_ref(),
                        inputs: s.inputs.as_slice(),
                        resolves: s.resolves.as_slice(),
                        preserves: s.preserves.as_slice(),
                    }),
                    dependencies,
                )
                .expect("[GfxContext] (create_render_pass) creation failed!")
        }
    }

    fn drop_render_pass(&self, rp: Self::RenderPassHandle) {
        unsafe {
            self.device.destroy_render_pass(rp);
        }
    }

    fn create_graphics_pipeline(
        &self,
        desc: GraphicsPipelineDescriptor<Self>,
        render_context: RenderContext<Self>,
    ) -> Self::PipelineHandle {
        self.plumber.create_pipeline(desc, render_context)
    }

    fn drop_pipeline(&self, pipeline: Self::PipelineHandle) {
        unsafe {
            self.device.destroy_graphics_pipeline(pipeline.0);
            self.device.destroy_pipeline_layout(pipeline.1);
        }
    }

    // fn get_surface_format(&self) -> TextureFormat {
    //     self.swapper.get_surface_format()
    // }

    fn compile_shader(&self, source: ShaderSource) -> Self::ShaderCode {
        self.plumber.compile_shader(source)
    }

    fn create_framebuffer<I>(
        &self,
        rp: &Self::RenderPassHandle,
        attachments: I,
        extent: Extent3D,
    ) -> Self::Framebuffer
    where
        I: IntoIterator,
        I::Item: Borrow<Self::ImageView>,
    {
        unsafe {
            self.device
                .create_framebuffer(rp, attachments, extent.convert())
                .expect("[GfxContext] (create_framebuffer) framebuffer creation failed")
        }
    }

    fn drop_framebuffer(&self, fb: Self::Framebuffer) {
        unsafe {
            self.device.destroy_framebuffer(fb);
        }
    }

    fn create_descriptor_layout<I>(&self, parts: I) -> Self::DescriptorLayout
    where
        I: IntoIterator<Item = render::resource::glue::MixturePart>,
    {
        self.pool.create_layout(parts)
    }

    fn drop_descriptor_layout(&self, handle: Self::DescriptorLayout) {
        self.pool.drop_layout(handle)
    }
    fn create_descriptor_set(&self, layout: &Mixture<Self>) -> Self::DescriptorSet {
        self.pool.allocate_set(layout)
    }

    fn drop_descriptor_set(&self, handle: Self::DescriptorSet) {
        self.pool.free_set(handle)
    }

    // fn single_shot_command(&self, should_wait: bool, cb: impl FnOnce(&mut Self::CommandEncoder)) {
    //     let mut queue = self.queues.graphics.lock();
    //     self.swapper.one_shot(should_wait, cb, queue.deref_mut())
    // }

    // fn new_frame(&self) -> (u32, Self::SwapchainImage) {
    //     match self.swapper.new_frame() {
    //         Ok(i) => i,
    //         Err(e) => {
    //             log::warn!("Ignorable error happened during new frame");
    //             log::warn!("{:#?}", e);
    //             // Try again, this will reconfigure the swapchain
    //             self.swapper.new_frame().unwrap()
    //         }
    //     }
    // }

    // fn end_frame(
    //     &self,
    //     swapchain_image: Self::SwapchainImage,
    //     frame_commands: Self::CommandBuffer,
    // ) {
    //     let mut graphics_queue = self.queues.graphics.lock();
    //     self.swapper
    //         .end_frame(swapchain_image, frame_commands, &mut graphics_queue)
    // }

    // fn render_command(&self, cb: impl FnOnce(&mut Self::CommandEncoder)) -> Self::CommandBuffer {
    //     self.swapper.render_command(cb)
    // }

    // fn swapchain_image_count(&self) -> usize {
    //     self.swapper.get_frames_in_flight()
    // }

    fn update_descriptor_set(
        &self,
        handle: &Self::DescriptorSet,
        writes: Vec<DescriptorWrite<Self>>,
    ) {
        self.pool.write_set(handle, writes)
    }

    fn wait_idle(&self) {
        self.device.wait_idle().expect("failed to wait idle");
    }

    fn create_graph(&self, surface: Self::SurfaceHandle) -> Self::GraphBuilder {
        let (surface, extent) = surface;
        GfxGraphBuilder::<B>::new(
            self.device.clone(),
            surface,
            extent,
            self.adapter.clone(),
            self.queues.clone(),
        )
    }
}

impl<B: Backend> Drop for GfxContext<B> {
    fn drop(&mut self) {
        self.device.wait_idle().expect("failed to wait idle");
    }
}

// Helper functions, mainly maps between our enums and gfx_hal's ones
fn get_buffer_usage(desc: &BufferDescriptor) -> gfx_hal::buffer::Usage {
    use gfx_hal::buffer::Usage;
    use render::resource::buffer::MemoryType;
    let usage = match desc.usage {
        BufferUsage::Uniform => Usage::UNIFORM,
        BufferUsage::Vertex => Usage::VERTEX,
        BufferUsage::Index => Usage::INDEX,
        BufferUsage::Staging => Usage::TRANSFER_SRC,
    };

    if desc.memory_type == MemoryType::DeviceLocal {
        usage | Usage::TRANSFER_DST
    } else {
        usage
    }
}
