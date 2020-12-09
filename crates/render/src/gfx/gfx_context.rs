use crate::context::GpuContext;
use crate::gfx::heapy::{AllocationIndex, Heapy};
use crate::resource::buffer::{BufferDescriptor, BufferUsage};
use crate::resource::pipeline::GraphicsPipelineDescriptor;
use crate::util::format::TextureFormat;
use bytemuck::Pod;
use gfx_hal::format::Format;
use gfx_hal::{device::Device, Backend};
use log::debug;
use std::{mem::ManuallyDrop, sync::Arc};

#[derive(Debug)]
struct Queues<B: Backend> {
    graphics: B::CommandQueue,
    compute: B::CommandQueue,
}

/// This is the GFX-hal implementation of the Rendering Context described in mod.rs
#[derive(Debug)]
pub struct GfxContext<B: Backend>
where
    B::Device: Send + Sync,
{
    instance: B::Instance,
    device: Arc<B::Device>,
    adapter: gfx_hal::adapter::Adapter<B>,
    surface: ManuallyDrop<B::Surface>,
    surface_format: TextureFormat,
    queues: Queues<B>,
    // Memory managment
    heapy: Heapy<B>,
    // Pipelines
}

impl<B: Backend> GfxContext<B> {
    pub fn new(window_handle: &impl raw_window_handle::HasRawWindowHandle) -> Self {
        use gfx_hal::{adapter::Gpu, queue::QueueFamily, Instance};

        let (instance, adapters, surface) = {
            let instance: B::Instance = B::Instance::create("City Builder Context", 1)
                .expect("failed to create a instance");

            let surface = unsafe {
                instance
                    .create_surface(window_handle)
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
        let adapter = adapters
            .into_iter()
            .find(|a| {
                a.queue_families.iter().any(|qf| {
                    qf.queue_type().supports_graphics() && surface.supports_queue_family(qf)
                })
            })
            .expect("couldn't find suitable adapter");
        debug!("Selected: {:?}", adapter.info);

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
                graphics: graphics_family.queues.remove(0),
                compute: compute_family.queues.remove(0),
            };

            (Arc::new(device), queues)
        };

        let surface_format = {
            use crate::gfx::compat::FromHalType;
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

        let heapy = Heapy::<B>::new(device.clone(), &adapter.physical_device);

        Self {
            instance,
            surface: ManuallyDrop::new(surface),
            surface_format,
            adapter,
            device,
            queues,
            heapy,
        }
    }
}

impl<B: Backend> GpuContext for GfxContext<B> {
    type BufferHandle = (B::Buffer, AllocationIndex);
    type PipelineHandle = B::GraphicsPipeline;

    fn create_buffer(&self, desc: &BufferDescriptor) -> Self::BufferHandle {
        unsafe {
            match self.device.create_buffer(desc.size, get_buffer_usage(desc)) {
                Ok(mut buffer) => {
                    let requirements = self.device.get_buffer_requirements(&buffer);
                    let allocation =
                        self.heapy
                            .alloc(requirements.size, desc.memory_type, Some(requirements));
                    self.heapy.bind_buffer(&allocation, &mut buffer);

                    use std::ops::Deref;
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

    fn create_graphics_pipeline(&self, desc: &GraphicsPipelineDescriptor) -> Self::PipelineHandle {
        unimplemented!()
    }

    fn drop_pipeline(&self, pipeline: Self::PipelineHandle) {
        unsafe {
            self.device.destroy_graphics_pipeline(pipeline);
        }
    }

    fn get_surface_format(&self) -> TextureFormat {
        self.surface_format.clone()
    }
}

impl<B: Backend> Drop for GfxContext<B> {
    fn drop(&mut self) {
        self.device.wait_idle().expect("failed to wait idle");
    }
}

// Helper functions, mainly maps between our enums and gfx_hal's ones
fn get_buffer_usage(desc: &BufferDescriptor) -> gfx_hal::buffer::Usage {
    use crate::resource::buffer::MemoryType;
    use gfx_hal::buffer::Usage;
    let usage = match desc.usage {
        BufferUsage::Uniform => Usage::UNIFORM,
        BufferUsage::Vertex => Usage::VERTEX,
        BufferUsage::Index => Usage::INDEX,
    };
    if desc.memory_type == MemoryType::DeviceLocal {
        usage | Usage::TRANSFER_DST
    } else {
        usage
    }
}
