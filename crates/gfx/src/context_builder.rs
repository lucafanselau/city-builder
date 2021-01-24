use gfx_hal::{
    adapter::{Adapter, DeviceType, Gpu},
    prelude::QueueFamily,
    Backend, Instance,
};
use log::*;
use parking_lot::Mutex;
use render::{context::GpuBuilder, prelude::GpuContext};
use std::sync::Arc;

use crate::{
    gfx_context::{GfxContext, Queues},
    heapy::Heapy,
    plumber::Plumber,
    pool::Pool,
};

pub struct GfxBuilder<B: Backend> {
    instance: B::Instance,
    surfaces: Vec<<GfxContext<B> as GpuContext>::SurfaceHandle>,
}

impl<B: Backend> GpuBuilder for GfxBuilder<B> {
    type Context = GfxContext<B>;

    fn new() -> Self {
        let instance = B::Instance::create("[gfx] backend", 1)
            .expect("[GfxBuilder] failed to create instance");

        Self {
            instance,
            surfaces: Vec::<<Self::Context as GpuContext>::SurfaceHandle>::new(),
        }
    }

    fn create_surface<W: raw_window_handle::HasRawWindowHandle>(
        &mut self,
        window: &W,
    ) -> <Self::Context as GpuContext>::SurfaceHandle {
        let surface = Arc::new(Mutex::new(unsafe {
            self.instance
                .create_surface(window)
                .expect("[GfxBuilder] failed to create surface")
        }));
        self.surfaces.push(surface.clone());
        surface
    }

    fn build(self) -> Self::Context {
        let GfxBuilder { instance, surfaces } = self;
        let adapters = instance.enumerate_adapters();

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
                    qf.queue_type().supports_graphics()
                        && surfaces.iter().all(|s| s.lock().supports_queue_family(qf))
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
                    surfaces
                        .iter()
                        .all(|s| s.lock().supports_queue_family(family))
                        && family.queue_type().supports_graphics()
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

                let Gpu {
                    device,
                    queue_groups,
                } = gpu;
                (device, queue_groups)
            };

            let mut queue_groups = queue_groups.into_iter();
            let mut graphics_family = queue_groups
                .find(|g| g.family == graphics_family.id())
                .expect("No graphics queue");
            let mut compute_family = queue_groups
                .find(|g| g.family == compute_family.id())
                .expect("No compute queue");

            let queues = Arc::new(Queues {
                graphics: Mutex::new(graphics_family.queues.remove(0)),
                graphics_family: graphics_family.family,
                compute: Mutex::new(compute_family.queues.remove(0)),
                compute_family: compute_family.family,
            });

            (Arc::new(device), queues)
        };

        let adapter = Arc::new(adapter);

        let heapy = Heapy::<B>::new(device.clone(), &adapter.physical_device);
        let plumber = Plumber::<B>::new(device.clone());
        // let swapper = Swapper::<B>::new(
        //     device.clone(),
        //     adapter.clone(),
        //     surface,
        //     surface_format,
        //     queues.graphics_family,
        // );

        let pool = Pool::<B>::new(device.clone());

        GfxContext {
            instance,
            adapter,
            device,
            queues,
            heapy,
            plumber,
            pool,
        }
    }
}
