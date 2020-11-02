use crate::context::GpuContext;
use crate::gfx::heapy::Heapy;
use crate::resource::buffer::BufferDescriptor;
use gfx_hal::{device::Device, Backend};
use log::debug;
use std::mem::ManuallyDrop;
use std::ops::Deref;
use std::sync::Arc;

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
    queues: Queues<B>,
    // Memory managment
    heapy: Heapy<B>,
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

        let heapy = Heapy::<B>::new(device.clone(), &adapter.physical_device);

        let mut buffer = unsafe {
            use gfx_hal::buffer::Usage;
            device
                .create_buffer(48, Usage::TRANSFER_DST | Usage::UNIFORM)
                .expect("failed to create test buffer")
        };

        let requirements = unsafe { device.get_buffer_requirements(&buffer) };

        use crate::gfx::heapy::MemoryType;
        let allocation = heapy.alloc(
            requirements.size,
            MemoryType::DeviceLocal,
            Some(requirements),
        );

        heapy.bind_buffer(allocation, &mut buffer);

        // Here the buffer has a beautiful life

        // now we should delete the buffer first and then deallocate
        unsafe {
            device.destroy_buffer(buffer);
        }
        heapy.deallocate(allocation);

        Self {
            instance,
            surface: ManuallyDrop::new(surface),
            adapter,
            device,
            queues,
            heapy,
        }
    }
}

impl<B: Backend> GpuContext for GfxContext<B> {
    type BufferHandle = B::Buffer;

    fn create_buffer(&self, desc: BufferDescriptor) -> Self::BufferHandle {
        unsafe {
            match self.device.create_buffer(desc.size, desc.usage) {
                Ok(buffer) => buffer,
                Err(err) => panic!(
                    "[GfxContext] failed to create buffer [{}]: {:#?}",
                    desc.name, err
                ),
            }
        }
    }

    fn drop_buffer(&self, buffer: Self::BufferHandle) {
        unsafe {
            self.device.destroy_buffer(buffer);
        }
    }
}
