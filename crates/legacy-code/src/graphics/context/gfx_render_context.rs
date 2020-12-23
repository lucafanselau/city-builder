use crate::graphics::context::render_context::{BufferError, RenderContext};
use crate::graphics::context::BufferHandle;
use gfx_hal::adapter::PhysicalDevice;
use gfx_hal::queue::{QueueFamily, QueueGroup};
use gfx_hal::{Backend, Instance};
use log::*;

use std::mem::ManuallyDrop;
use std::rc::Rc;
use std::sync::Arc;

/// RenderContext Implementation for gfx-hal
pub struct GfxRenderContext<B: Backend>
where
    B::Device: Send + Sync,
{
    instance: B::Instance,
    surface: ManuallyDrop<B::Surface>,
    adapter: gfx_hal::adapter::Adapter<B>,
    device: Arc<B::Device>,
    queue_group: Vec<QueueGroup<B>>,
}

impl<B: Backend> GfxRenderContext<B> {
    pub fn new(name: &str, window: Rc<winit::window::Window>) -> Self {
        let (instance, adapters, surface) = {
            let instance: B::Instance =
                B::Instance::create(name, 1).expect("failed to create a instance");

            let surface = unsafe {
                use std::borrow::Borrow;
                instance
                    .create_surface(window.borrow() as &winit::window::Window)
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

        let (device, queue_group) = {
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
                .find(|family| family.queue_type().supports_compute())
                .expect("couldn't find compute queue_family");

            let gpu = unsafe {
                use gfx_hal::adapter::PhysicalDevice;

                adapter
                    .physical_device
                    .open(
                        &[(graphics_family, &[1.0; 1]), (compute_family, &[1.0; 1])],
                        gfx_hal::Features::empty(),
                    )
                    .expect("failed to open device")
            };

            (Arc::new(gpu.device), gpu.queue_groups)
        };

        Self {
            instance,
            surface: ManuallyDrop::new(surface),
            adapter,
            device,
            queue_group,
        }
    }
}

impl<B: Backend> RenderContext for GfxRenderContext<B> {
    fn create_buffer(&self) -> Result<BufferHandle, BufferError> {
        unimplemented!();
    }
}
