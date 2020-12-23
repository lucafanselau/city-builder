use crate::compat::ToHalType;
use crate::gfx_command::GfxCommand;
use gfx_hal::command::{CommandBuffer, CommandBufferFlags, Level};
use gfx_hal::device::{Device, WaitFor};
use gfx_hal::pool::{CommandPool, CommandPoolCreateFlags};
use gfx_hal::prelude::CommandQueue;
use gfx_hal::queue::{QueueFamilyId, Submission};
use gfx_hal::window::{Extent2D, PresentationSurface, SurfaceCapabilities};
use gfx_hal::Backend;
use gfx_hal::{adapter::Adapter, pso};
use parking_lot::{Mutex, MutexGuard, RwLock};
use pso::PipelineStage;
use render::util::format::TextureFormat;
use std::convert::TryInto;
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Debug)]
enum FrameStatus {
    Active,
    Inactive,
}

#[derive(Debug)]
struct FrameSynchronization<B: Backend> {
    submission_fence: B::Fence,
    rendering_complete: B::Semaphore,
    in_use_command: Option<B::CommandBuffer>,
}

impl<B: Backend> FrameSynchronization<B> {
    unsafe fn create(device: &Arc<B::Device>) -> Self {
        Self {
            submission_fence: device
                .create_fence(true)
                .expect("[Swapper] failed to create simple sync primitive"),
            rendering_complete: device
                .create_semaphore()
                .expect("[Swapper] failed to create simple sync primitive"),
            in_use_command: None,
        }
    }
}

/// This turned out to be a really simple "renderer", like synchronization and command buffers
#[derive(Debug)]
pub struct Swapper<B: Backend> {
    device: Arc<B::Device>,
    surface: ManuallyDrop<RwLock<B::Surface>>,
    surface_format: TextureFormat,
    /// None before first configure swapchain
    surface_extent: RwLock<Option<Extent2D>>,
    should_configure_swapchain: AtomicBool,
    adapter: Arc<Adapter<B>>,

    /// None -> No Active Frame (outside begin_frame(), end_frame()), Some -> Inside with frame idx
    current_frame: RwLock<(u32, FrameStatus)>,
    frames_in_flight: u32,
    frames: Vec<ManuallyDrop<Mutex<FrameSynchronization<B>>>>,

    // Command Managment -> This is not intended to be a command pool manager
    // We will need a more sophisticated solution in the future, but for now, to make
    // things work, here we go
    command_pool: ManuallyDrop<Mutex<B::CommandPool>>,
}

const TIMEOUT: u64 = 1_000_000_000u64;

type SwapchainImage<B> = <<B as Backend>::Surface as PresentationSurface<B>>::SwapchainImage;

impl<B: Backend> Swapper<B> {
    pub fn new(
        device: Arc<B::Device>,
        adapter: Arc<Adapter<B>>,
        surface: B::Surface,
        surface_format: TextureFormat,
        graphics_family: QueueFamilyId,
    ) -> Self {
        let frames_in_flight = 3u32;
        let mut frames = Vec::with_capacity(frames_in_flight.try_into().unwrap());
        unsafe {
            for _ in 0..frames_in_flight {
                frames.push(ManuallyDrop::new(Mutex::new(
                    FrameSynchronization::<B>::create(&device),
                )));
            }
        }

        let command_pool = unsafe {
            device
                .create_command_pool(graphics_family, CommandPoolCreateFlags::RESET_INDIVIDUAL)
                .expect("[Swapper] failed to create command_pool")
        };

        Self {
            device,
            surface: ManuallyDrop::new(RwLock::new(surface)),
            surface_format,
            surface_extent: RwLock::new(None),
            should_configure_swapchain: AtomicBool::new(true),
            adapter,
            current_frame: RwLock::new((0, FrameStatus::Inactive)),
            frames_in_flight,
            frames,
            command_pool: ManuallyDrop::new(Mutex::new(command_pool)),
        }
    }

    fn configure_swapchain(&self) -> anyhow::Result<()> {
        if self.should_configure_swapchain.load(Ordering::Relaxed) {
            use gfx_hal::window::{Surface, SwapchainConfig};

            // First we need to wait for all frames to finish
            let wait_timeout_ns = 1_000_000_000;
            unsafe {
                let frames: Vec<MutexGuard<FrameSynchronization<B>>> =
                    self.frames.iter().map(|f| f.lock()).collect();
                let fences: Vec<&B::Fence> = frames.iter().map(|f| &f.submission_fence).collect();
                self.device
                    .wait_for_fences(fences, WaitFor::All, wait_timeout_ns)
            }?;

            {
                let mut surface = self.surface.deref().write();
                let caps: SurfaceCapabilities = surface.capabilities(&self.adapter.physical_device);

                let extent = caps
                    .current_extent
                    // TODO: REMOVE!!!
                    .unwrap_or(self.surface_extent.read().unwrap_or(Extent2D {
                        width: 1600,
                        height: 900,
                    }));

                let mut swapchain_config = SwapchainConfig::from_caps(
                    &caps,
                    self.surface_format.clone().convert(),
                    extent,
                );
                // This seems to fix some fullscreen slowdown on macOS.
                if caps.image_count.contains(&3) {
                    swapchain_config.image_count = 3;
                }

                {
                    *self.surface_extent.write() = Some(swapchain_config.extent);
                }

                unsafe { surface.configure_swapchain(&self.device, swapchain_config)? };
            };

            self.should_configure_swapchain
                .store(false, Ordering::Relaxed)
        }
        Ok(())
    }

    pub fn get_surface_format(&self) -> TextureFormat {
        self.surface_format.clone()
    }

    pub fn new_frame(&self) -> anyhow::Result<(u32, SwapchainImage<B>)> {
        let frame_idx = {
            let mut current_frame = self.current_frame.write();
            match current_frame.1 {
                FrameStatus::Active => {
                    panic!("[Swapper] called new frame, but there is still an active frame")
                }
                FrameStatus::Inactive => {
                    current_frame.1 = FrameStatus::Active;
                    current_frame.0
                }
            }
        };

        self.configure_swapchain()?;

        // Wait for the in flight image of the current index
        unsafe {
            let render_timeout_ns = 1_000_000_000;

            let mut this_frame = self.frames.get(frame_idx as usize).unwrap().lock();

            self.device
                .wait_for_fence(&this_frame.submission_fence, render_timeout_ns)?;
            self.device.reset_fence(&this_frame.submission_fence)?;

            // Now we also need to delete the command buffer in use
            if this_frame.in_use_command.is_some() {
                let command_buffer = this_frame.in_use_command.take().unwrap();
                self.command_pool.lock().free(vec![command_buffer]);
            }
        };

        // Acquire Image
        Ok(unsafe {
            // We refuse to wait more than a second, to avoid hanging.
            let acquire_timeout_ns = 1_000_000_000;

            let mut surface = self.surface.write();

            match surface.deref_mut().acquire_image(acquire_timeout_ns) {
                Ok((image, _)) => Ok((frame_idx, image)),
                Err(e) => {
                    self.should_configure_swapchain
                        .store(true, Ordering::Relaxed);
                    Err(e)
                }
            }?
        })
    }

    pub fn render_command(&self, cb: impl FnOnce(&mut GfxCommand<B>)) -> B::CommandBuffer {
        // create the command buffer
        let mut command = unsafe { self.command_pool.lock().allocate_one(Level::Primary) };

        // Start the command buffer
        unsafe {
            command.begin_primary(CommandBufferFlags::ONE_TIME_SUBMIT);
        }

        // Command Encoder Abstraction
        let mut gfx_command = GfxCommand::<B>::new(command);
        cb(&mut gfx_command);

        // Finish the command buffer
        let mut command = gfx_command.into_inner();
        unsafe {
            command.finish();
        }
        command
    }

    pub fn end_frame(
        &self,
        swapchain_image: SwapchainImage<B>,
        frame_commands: B::CommandBuffer,
        graphics_queue: &mut B::CommandQueue,
    ) {
        let frame_idx = {
            let mut current_frame = self.current_frame.write();
            match current_frame.1 {
                FrameStatus::Active => {
                    let current_idx = current_frame.0;
                    current_frame.0 = (current_frame.0 + 1) % self.frames_in_flight;
                    current_frame.1 = FrameStatus::Inactive;
                    current_idx
                }
                FrameStatus::Inactive => {
                    panic!("[Swapper] called new frame, but there is still an active frame")
                }
            }
        };

        let mut this_frame = self.frames.get(frame_idx as usize).unwrap().lock();

        unsafe {
            let submission = Submission {
                command_buffers: vec![&frame_commands],
                wait_semaphores: None,
                signal_semaphores: vec![&this_frame.rendering_complete],
            };

            graphics_queue.submit(submission, Some(&this_frame.submission_fence));
        }

        this_frame.in_use_command = Some(frame_commands);

        unsafe {
            let mut surface = self.surface.write();
            // NOTE(luca): Currently we do not think about suboptimal swapchains
            let result = graphics_queue.present(
                &mut surface,
                swapchain_image,
                Some(&this_frame.rendering_complete),
            );

            if result.is_err() {
                self.should_configure_swapchain
                    .store(true, Ordering::Relaxed);
            }
        }
    }

    pub fn get_frames_in_flight(&self) -> usize {
        self.frames_in_flight as usize
    }

    pub fn one_shot(
        &self,
        should_wait: bool,
        cb: impl FnOnce(&mut GfxCommand<B>),
        queue: &mut B::CommandQueue,
    ) {
        let command = self.render_command(cb);

        let fence = if should_wait {
            Some(
                self.device
                    .create_fence(false)
                    .expect("[Swapper] (one_shot) failed to create fence"),
            )
        } else {
            None
        };

        unsafe {
            queue.submit(
                Submission {
                    command_buffers: vec![&command],
                    wait_semaphores: Vec::<(&B::Semaphore, PipelineStage)>::new(),
                    signal_semaphores: Vec::<&B::Semaphore>::new(),
                },
                fence.as_ref(),
            );
        }

        if let Some(fence) = fence {
            unsafe {
                self.device
                    .wait_for_fence(&fence, TIMEOUT)
                    .expect("[Swapper] (one_shot) failed to wait fence")
            };
        }
    }
}

impl<B: Backend> Drop for Swapper<B> {
    fn drop(&mut self) {
        for mut frame in self.frames.drain(..) {
            unsafe {
                let frame = ManuallyDrop::take(&mut frame).into_inner();
                self.device.destroy_semaphore(frame.rendering_complete);
                self.device.destroy_fence(frame.submission_fence);
            }
        }
        unsafe {
            let pool = ManuallyDrop::take(&mut self.command_pool).into_inner();
            self.device.destroy_command_pool(pool);
        }
    }
}
