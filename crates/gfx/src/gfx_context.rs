use crate::{compat::{HalCompatibleSubpassDescriptor, ToHalType}, graph::GfxGraph};
use crate::gfx_command::GfxCommand;
use crate::heapy::{AllocationIndex, Heapy};
use crate::plumber::Plumber;
use crate::swapper::Swapper;
use bytemuck::Pod;
use gfx_hal::format::Format;
use gfx_hal::pass::{Attachment, SubpassDependency, SubpassDesc};
use gfx_hal::queue::QueueFamilyId;
use gfx_hal::window::PresentationSurface;
use gfx_hal::{
    adapter::{Adapter, DeviceType},
    Instance,
};
use gfx_hal::{device::Device, Backend};
use log::{debug, info};
use parking_lot::Mutex;
use raw_window_handle::HasRawWindowHandle;
use render::resource::buffer::{BufferDescriptor, BufferUsage};
use render::resource::frame::Extent3D;
use render::resource::glue::Mixture;
use render::resource::pipeline::{GraphicsPipelineDescriptor, RenderContext, ShaderSource};
use render::resource::render_pass::RenderPassDescriptor;
use render::util::format::TextureFormat;
use render::{context::GpuContext, resource::glue::DescriptorWrite};
use std::{borrow::Borrow, ops::DerefMut};
use std::{ops::Deref, sync::Arc};

use super::pool::{LayoutHandle, Pool, SetHandle};

#[derive(Debug)]
struct Queues<B: Backend> {
    graphics: Mutex<B::CommandQueue>,
    graphics_family: QueueFamilyId,
    compute: Mutex<B::CommandQueue>,
    compute_family: QueueFamilyId,
}

use gfx_backend_vulkan as graphics_backend;
pub type Context = GfxContext<graphics_backend::Backend>;

/// This is the GFX-hal implementation of the Rendering Context described in mod.rs
#[derive(Debug)]
pub struct GfxContext<B: Backend>
where
    B::Device: Send + Sync,
{
    instance: B::Instance,
    device: Arc<B::Device>,
    adapter: Arc<gfx_hal::adapter::Adapter<B>>,
    queues: Queues<B>,
    // Memory managment
    heapy: Heapy<B>,
    // Pipelines
    plumber: Plumber<B>,
    // Swapchain
    swapper: Swapper<B>,
    // Pool -> Descriptor Sets
    pool: Pool<B>,
}

impl<B: Backend> GfxContext<B> {
    pub fn new(window: &impl HasRawWindowHandle) -> Self {
        use gfx_hal::{adapter::Gpu, queue::QueueFamily};

        let (instance, adapters, surface) = {
            let instance: B::Instance = B::Instance::create("City Builder Context", 1)
                .expect("failed to create a instance");

            let surface = unsafe {
                instance
                    .create_surface(window)
                    .expect("failed to create surface")
            };

            let adapters = instance.enumerate_adapters();

            (instance, adapters, surface)
        };

        // Select Physical Device
        debug!("Found Adapters: ");
        for adapter in &adapters {
            debug!("{:?}", adapter.info);
        }

        use gfx_hal::window::Surface;
        // our adapter selection is a bit more sophisticated
        // atm we just check if we have a graphics card that is capable of rendering to our screen
        let rated_adapters: Vec<(u64, Adapter<B>)> = adapters
            .into_iter()
            .filter(|a| {
                a.queue_families.iter().any(|qf| {
                    qf.queue_type().supports_graphics() && surface.supports_queue_family(qf)
                })
            })
            .map(|a| {
                let mut score = 0u64;
                if a.queue_families
                    .iter()
                    .any(|qf| qf.queue_type().supports_compute())
                {
                    score += 20u64;
                }
                if a.info.device_type == DeviceType::DiscreteGpu {
                    score += 1000u64;
                }
                (score, a)
            })
            .collect();

        let (score, adapter) = rated_adapters
            .into_iter()
            .max_by(|a, b| a.0.cmp(&b.0))
            .expect("[GfxContext] failed to find suitable gpu");

        info!("Selected: {:?} (with score: {})", adapter.info, score);

        let (device, queues) = {
            // need to find the queue_family
            let families = &adapter.queue_families;

            let graphics_family = families
                .iter()
                .find(|family| {
                    surface.supports_queue_family(family) && family.queue_type().supports_graphics()
                })
                .expect("couldn't find graphics queue_family");

            let compute_family = families
                .iter()
                .find(|family| {
                    family.queue_type().supports_compute() && family.id() != graphics_family.id()
                })
                .expect("couldn't find compute queue_family");

            let (device, queue_groups) = unsafe {
                use gfx_hal::adapter::PhysicalDevice;

                let gpu = adapter
                    .physical_device
                    .open(
                        &[(graphics_family, &[1.0; 1]), (compute_family, &[1.0; 1])],
                        gfx_hal::Features::empty(),
                    )
                    .expect("failed to open device");

                match gpu {
                    Gpu {
                        device,
                        queue_groups,
                    } => (device, queue_groups),
                }
            };

            let mut queue_groups = queue_groups.into_iter();
            let mut graphics_family = queue_groups
                .find(|g| g.family == graphics_family.id())
                .expect("No graphics queue");
            let mut compute_family = queue_groups
                .find(|g| g.family == compute_family.id())
                .expect("No compute queue");

            let queues = Queues {
                graphics: Mutex::new(graphics_family.queues.remove(0)),
                graphics_family: graphics_family.family,
                compute: Mutex::new(compute_family.queues.remove(0)),
                compute_family: compute_family.family,
            };

            (Arc::new(device), queues)
        };

        let surface_format = {
            use crate::compat::FromHalType;
            use gfx_hal::format::ChannelType;

            let supported_formats = surface
                .supported_formats(&adapter.physical_device)
                .unwrap_or(vec![]);

            let default_format = *supported_formats.get(0).unwrap_or(&Format::Rgba8Srgb);

            let hal_format = supported_formats
                .into_iter()
                .find(|format| format.base_format().1 == ChannelType::Srgb)
                .unwrap_or(default_format);

            hal_format
                .convert()
                .expect("[GfxContext] failed to convert surface format")
        };

        let adapter = Arc::new(adapter);

        let heapy = Heapy::<B>::new(device.clone(), &adapter.physical_device);
        let plumber = Plumber::<B>::new(device.clone());
        let swapper = Swapper::<B>::new(
            device.clone(),
            adapter.clone(),
            surface,
            surface_format,
            queues.graphics_family,
        );

        let pool = Pool::<B>::new(device.clone());

        Self {
            instance,
            adapter,
            device,
            queues,
            heapy,
            plumber,
            swapper,
            pool,
        }
    }
}

impl<B: Backend> GpuContext for GfxContext<B> {
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
    type ContextGraph = GfxGraph<B>;

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

    fn compile_shader(&self, source: ShaderSource) -> Self::ShaderCode {
        self.plumber.compile_shader(source)
    }

    fn get_surface_format(&self) -> TextureFormat {
        self.swapper.get_surface_format()
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

    fn update_descriptor_set(
        &self,
        handle: &Self::DescriptorSet,
        writes: Vec<DescriptorWrite<Self>>,
    ) {
        self.pool.write_set(handle, writes)
    }

    fn single_shot_command(&self, should_wait: bool, cb: impl FnOnce(&mut Self::CommandEncoder)) {
        let mut queue = self.queues.graphics.lock();
        self.swapper.one_shot(should_wait, cb, queue.deref_mut())
    }

    fn new_frame(&self) -> (u32, Self::SwapchainImage) {
        match self.swapper.new_frame() {
            Ok(i) => i,
            Err(e) => {
                log::warn!("Ignorable error happened during new frame");
                log::warn!("{:#?}", e);
                // Try again, this will reconfigure the swapchain
                self.swapper.new_frame().unwrap()
            }
        }
    }

    fn end_frame(
        &self,
        swapchain_image: Self::SwapchainImage,
        frame_commands: Self::CommandBuffer,
    ) {
        let mut graphics_queue = self.queues.graphics.lock();
        self.swapper
            .end_frame(swapchain_image, frame_commands, &mut graphics_queue)
    }

    fn render_command(&self, cb: impl FnOnce(&mut Self::CommandEncoder)) -> Self::CommandBuffer {
        self.swapper.render_command(cb)
    }

    fn wait_idle(&self) {
        self.device.wait_idle().expect("failed to wait idle");
    }

    fn swapchain_image_count(&self) -> usize {
        self.swapper.get_frames_in_flight()
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
