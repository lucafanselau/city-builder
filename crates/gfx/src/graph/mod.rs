use core::anyhow;
use std::{
    any::Any,
    borrow::Borrow,
    mem::ManuallyDrop,
    ops::Deref,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use app::{Resources, World};
use gfx_hal::{
    adapter::Adapter,
    command::{CommandBuffer, CommandBufferFlags, Level},
    device::{Device, WaitFor},
    image::Extent,
    pool::CommandPool,
    prelude::CommandQueue,
    queue::Submission,
    window::{PresentMode, PresentationSurface, Surface, SurfaceCapabilities, SwapchainConfig},
    Backend,
};
use parking_lot::{Mutex, MutexGuard, RwLock};
use render::{
    graph::{nodes::callbacks::FrameData, Graph},
    prelude::CommandEncoder,
    resource::{
        frame::{Clear, Extent2D},
        pipeline::{Rect, Viewport},
    },
    util::format::TextureFormat,
};

use crate::{
    command::GfxCommand,
    compat::ToHalType,
    context::{GfxContext, Queues},
    heapy::Heapy,
};

pub mod attachment;
use self::{
    attachment::{AttachmentIndex, GfxGraphAttachment},
    builder::GfxGraphBuilder,
    nodes::GfxNode,
};

pub mod builder;
pub mod nodes;

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
    pass_data: Vec<Box<dyn Any>>,
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
            pass_data: Vec::new(),
        }
    }
}

// Data that needs to be accessable from both the GraphBuilder and the Graph itself
pub struct GraphData<B: Backend> {
    device: Arc<B::Device>,
    surface: Arc<Mutex<B::Surface>>,
    adapter: Arc<Adapter<B>>,
    queues: Arc<Queues<B>>,
    heapy: Arc<Heapy<B>>,

    // Static swapchain data
    depth_format: TextureFormat,
    surface_format: TextureFormat,
    surface_extent: RwLock<Extent2D>,
    frames_in_flight: u32,
    // Pool
    command_pool: ManuallyDrop<Mutex<B::CommandPool>>,
}

pub struct GfxGraph<B: Backend> {
    attachments: Vec<GfxGraphAttachment<B>>,
    nodes: Vec<GfxNode<B>>,

    data: GraphData<B>,

    // Dynamic Render Data
    should_configure_swapchain: AtomicBool,
    current_frame: RwLock<(u32, FrameStatus)>,
    frames: Mutex<Vec<ManuallyDrop<FrameSynchronization<B>>>>,
}

type SwapchainImage<B> = <<B as Backend>::Surface as PresentationSurface<B>>::SwapchainImage;

impl<B: Backend> GfxGraph<B> {
    fn configure_swapchain(&self) -> anyhow::Result<()> {
        if self.should_configure_swapchain.load(Ordering::Relaxed) {
            // First we need to wait for all frames to finish
            let wait_timeout_ns = 1_000_000_000;
            unsafe {
                let mut frames = self.frames.lock();
                let frame_result = {
                    let fences: Vec<&B::Fence> =
                        frames.iter().map(|f| &f.submission_fence).collect();
                    self.data
                        .device
                        .wait_for_fences(fences, WaitFor::All, wait_timeout_ns)
                };
                // Reset frame data
                frames.iter_mut().map(|f| f.pass_data = Vec::new());
                frame_result
            }?;

            {
                let mut surface = self.data.surface.lock();
                let caps: SurfaceCapabilities =
                    surface.capabilities(&self.data.adapter.physical_device);
                // log::info!("available modes: {:?}", caps.present_modes);

                let mut swapchain_config = SwapchainConfig::from_caps(
                    &caps,
                    self.data.surface_format.clone().convert(),
                    self.data.surface_extent.read().clone().convert(),
                );
                // This seems to fix some fullscreen slowdown on macOS.
                if caps.image_count.contains(&3) {
                    swapchain_config.image_count = 3;
                }
                log::info!("Available Image Counts: {:?}", caps.image_count);
                log::info!("Image Count {}", swapchain_config.image_count);

                // log::info!("swapchain mode: {:?}", swapchain_config.present_mode);
                swapchain_config.present_mode = PresentMode::IMMEDIATE;

                unsafe { surface.configure_swapchain(&self.data.device, swapchain_config)? };
            };

            self.should_configure_swapchain
                .store(false, Ordering::Relaxed)
        }
        Ok(())
    }

    pub fn new_frame(&self) -> anyhow::Result<(u32, SwapchainImage<B>)> {
        self.configure_swapchain()?;

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

        // Wait for the in flight image of the current index
        unsafe {
            let render_timeout_ns = 1_000_000_000;

            let mut frames = self.frames.lock();
            let this_frame = frames.get_mut(frame_idx as usize).unwrap();

            self.data
                .device
                .wait_for_fence(&this_frame.submission_fence, render_timeout_ns)?;
            self.data.device.reset_fence(&this_frame.submission_fence)?;

            // And free the frame data
            this_frame.pass_data = Vec::new();

            // Now we also need to delete the command buffer in use
            if this_frame.in_use_command.is_some() {
                let command_buffer = this_frame.in_use_command.take().unwrap();
                self.data.command_pool.lock().free(vec![command_buffer]);
            }
        };

        // Acquire Image
        Ok(unsafe {
            // We refuse to wait more than a second, to avoid hanging.
            let acquire_timeout_ns = 1_000_000_000;

            match self.data.surface.lock().acquire_image(acquire_timeout_ns) {
                Ok((image, _)) => Ok((frame_idx, image)),
                Err(e) => {
                    self.should_configure_swapchain
                        .store(true, Ordering::Relaxed);
                    Err(e)
                }
            }?
        })
    }
}

impl<B: Backend> Graph for GfxGraph<B> {
    type Context = GfxContext<B>;
    type AttachmentIndex = AttachmentIndex;
    type Builder = GfxGraphBuilder<B>;

    fn execute(&mut self, world: &World, resources: &Resources) {
        {
            // Check for resize events
            let resize_events = resources
                .get::<app::Events<window::events::WindowResize>>()
                .expect("[GfxGraph] (execute) failed to get resize event");

            // Get the last window extent
            if let Some(&window::events::WindowResize(new_extent)) = resize_events.iter().last() {
                let new_dimension = Extent2D {
                    width: new_extent.width,
                    height: new_extent.height,
                };
                *self.data.surface_extent.write() = new_dimension.clone();
                self.should_configure_swapchain
                    .store(true, Ordering::Relaxed);

                // This will also force us to recreate custom attachments
                for a in self.attachments.iter_mut() {
                    a.rebuild(
                        self.data.device.deref(),
                        self.data.heapy.deref(),
                        new_dimension.clone(),
                        self.nodes.iter().map(|n| match n {
                            GfxNode::PassNode(n) => &n.graph_node,
                        }),
                    );
                }
            }
        }

        self.configure_swapchain()
            .expect("[GfxGraph] failed to configure swapchain");

        let (index, swapchain_image) = match self.new_frame() {
            Ok(i) => i,
            Err(e) => {
                log::warn!("Ignorable error happened during new frame");
                log::warn!("{:#?}", e);
                // Try again, this will reconfigure the swapchain
                self.new_frame().unwrap()
            }
        };

        let command = {
            // create the command buffer
            let mut command = unsafe { self.data.command_pool.lock().allocate_one(Level::Primary) };

            // Start the command buffer
            unsafe {
                command.begin_primary(CommandBufferFlags::ONE_TIME_SUBMIT);
            }

            // Command Encoder Abstraction
            let mut gfx_command = GfxCommand::<B>::new(command);
            let extent = self.data.surface_extent.read().clone();
            let extent3d = Extent {
                width: extent.width,
                height: extent.height,
                depth: 1,
            };

            // TODO: Execute all passes
            for node in self.nodes.iter() {
                // TODO: Expand to diffrent nodes
                let GfxNode::PassNode(node) = node;
                let framebuffer = unsafe {
                    let mut attachments = node.graph_node.output_attachments.clone();
                    attachments.extend(node.graph_node.input_attachments.clone());
                    if let Some(a) = &node.graph_node.depth_attachment {
                        attachments.push(a.clone())
                    };
                    let attachments = attachments.iter().map(|a| match a.index {
                        AttachmentIndex::Backbuffer => swapchain_image.borrow(),
                        AttachmentIndex::Custom(id) => {
                            &self
                                .attachments
                                .iter()
                                .find(|a| a.desc.id == id)
                                .expect("[GfxGraph] (execute) failed to load custom attachment")
                                .image_view
                        }
                    });
                    self.data
                        .device
                        .create_framebuffer(&node.render_pass, attachments, extent3d)
                        .expect("[GfxGraph] failed to create viewport")
                };

                let viewport = Viewport {
                    rect: Rect {
                        x: 0,
                        y: 0,
                        width: extent.width as i16,
                        height: extent.height as i16,
                    },
                    depth: 0.0..1.0,
                };

                gfx_command.begin_render_pass(
                    &node.render_pass,
                    &framebuffer,
                    viewport.clone().rect,
                    // TODO: Clear Values
                    vec![Clear::Color(0.2, 0.5, 0.1, 1.0), Clear::Depth(1.0, 0)],
                );

                // Execute Callback
                let frame_data = FrameData {
                    cmd: &mut gfx_command,
                    frame_index: index,
                    viewport: viewport.clone(),
                };
                match node
                    .graph_node
                    .callbacks
                    .borrow_mut()
                    .run(frame_data, world, resources)
                {
                    Ok(Some(data)) => {
                        let mut frames = self.frames.lock();
                        frames.get_mut(index as usize).unwrap().pass_data.push(data);
                    }
                    Ok(None) => (),
                    Err(e) => {
                        panic!(
                            "[GfxGraph] execution failed during pass node: {}, with error: {}",
                            node.graph_node.name, e
                        )
                    }
                }

                // and end render pass
                gfx_command.end_render_pass()
            }
            // self.nodes.iter().
            // cb(&mut gfx_command);

            // Finish the command buffer
            let mut command = gfx_command.into_inner();
            unsafe {
                command.finish();
            }
            command
        };

        let frame_idx = {
            let mut current_frame = self.current_frame.write();
            match current_frame.1 {
                FrameStatus::Active => {
                    let current_idx = current_frame.0;
                    current_frame.0 = (current_frame.0 + 1) % self.data.frames_in_flight;
                    current_frame.1 = FrameStatus::Inactive;
                    current_idx
                }
                FrameStatus::Inactive => {
                    panic!("[Swapper] called new frame, but there is still an active frame")
                }
            }
        };

        let mut frames = self.frames.lock();
        let this_frame = frames.get_mut(frame_idx as usize).unwrap();
        let mut graphics_queue = self.data.queues.graphics.lock();

        unsafe {
            let submission = Submission {
                command_buffers: vec![&command],
                wait_semaphores: None,
                signal_semaphores: vec![&this_frame.rendering_complete],
            };
            graphics_queue.submit(submission, Some(&this_frame.submission_fence));
        }

        this_frame.in_use_command = Some(command);

        unsafe {
            let surface = &mut self.data.surface.lock();

            // NOTE(luca): Currently we do not think about suboptimal swapchains
            let result = graphics_queue.present(
                surface,
                swapchain_image,
                Some(&this_frame.rendering_complete),
            );

            if let Err(_e) = result {
                // log::warn!("Recovarable Error happened");
                // log::warn!("{:#?}", e);
                self.should_configure_swapchain
                    .store(true, Ordering::Relaxed);
            }
        }
    }

    fn into_builder(self) -> Self::Builder {
        todo!()
    }
}

// TODO: Drop the custom nodes
impl<B: Backend> Drop for GfxGraph<B> {
    fn drop(&mut self) {
        self.data
            .device
            .wait_idle()
            .expect("[Graph] failed to wait_idle");
    }
}
