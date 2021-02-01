use std::{
    borrow::{Borrow, Cow},
    convert::TryInto,
    mem::ManuallyDrop,
    ops::Deref,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use app::{Resources, World};
use generational_arena::Arena;
use gfx_hal::{
    adapter::Adapter,
    command::{CommandBuffer, CommandBufferFlags, Level},
    device::{Device, WaitFor},
    format::{ChannelType, Format},
    image::Extent,
    pool::{CommandPool, CommandPoolCreateFlags},
    prelude::CommandQueue,
    queue::Submission,
    window::{PresentMode, PresentationSurface, Surface, SurfaceCapabilities, SwapchainConfig},
    Backend,
};
use parking_lot::{Mutex, MutexGuard, RwLock};
use render::{
    graph::{attachment::GraphAttachment, node::Node, nodes::callbacks::FrameData, Graph},
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
};

pub mod attachment;
use self::{attachment::AttachmentIndex, nodes::GfxNode};

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

pub struct GfxGraph<B: Backend> {
    device: Arc<B::Device>,
    surface: Arc<Mutex<B::Surface>>,
    adapter: Arc<Adapter<B>>,
    attachments: Arena<GraphAttachment>,
    nodes: Arena<GfxNode<B>>,

    // Rendering related stuff
    surface_format: TextureFormat,
    surface_extent: RwLock<Extent2D>,
    queues: Arc<Queues<B>>,
    should_configure_swapchain: AtomicBool,

    // Frame Managment
    current_frame: RwLock<(u32, FrameStatus)>,
    frames_in_flight: u32,
    frames: Vec<ManuallyDrop<Mutex<FrameSynchronization<B>>>>,

    // Pool
    command_pool: ManuallyDrop<Mutex<B::CommandPool>>,
}

type SwapchainImage<B> = <<B as Backend>::Surface as PresentationSurface<B>>::SwapchainImage;

impl<B: Backend> GfxGraph<B> {
    pub(crate) fn new(
        device: Arc<B::Device>,
        surface: Arc<Mutex<B::Surface>>,
        extent: Extent2D,
        adapter: Arc<Adapter<B>>,
        queues: Arc<Queues<B>>,
    ) -> Self {
        let surface_format = {
            use crate::compat::FromHalType;

            let surface = surface.lock();
            let supported_formats = surface
                .supported_formats(&adapter.physical_device)
                .unwrap_or_default();

            let default_format = *supported_formats.get(0).unwrap_or(&Format::Rgba8Srgb);

            let hal_format = supported_formats
                .into_iter()
                .find(|format| format.base_format().1 == ChannelType::Srgb)
                .unwrap_or(default_format);

            hal_format
                .convert()
                .expect("[GfxGraph] failed to convert surface format")
        };

        let frames_in_flight = 3u32;
        let mut frames = Vec::with_capacity(frames_in_flight.try_into().unwrap());
        unsafe {
            for _ in 0..frames_in_flight {
                frames.push(ManuallyDrop::new(Mutex::new(
                    FrameSynchronization::<B>::create(&device),
                )));
            }
        }

        let graphics_family = queues.graphics_family;
        let command_pool = unsafe {
            device
                .create_command_pool(graphics_family, CommandPoolCreateFlags::RESET_INDIVIDUAL)
                .expect("[Swapper] failed to create command_pool")
        };

        Self {
            device,
            surface,
            adapter,
            attachments: Arena::new(),
            nodes: Arena::new(),
            surface_format,
            surface_extent: RwLock::new(extent),
            queues,
            should_configure_swapchain: AtomicBool::from(true),
            current_frame: RwLock::new((0, FrameStatus::Inactive)),
            frames_in_flight,
            frames,
            command_pool: ManuallyDrop::new(Mutex::new(command_pool)),
        }
    }

    pub fn get_surface_format(&self) -> TextureFormat {
        self.surface_format.clone()
    }

    fn configure_swapchain(&self) -> anyhow::Result<()> {
        if self.should_configure_swapchain.load(Ordering::Relaxed) {
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
                let mut surface = self.surface.lock();
                let caps: SurfaceCapabilities = surface.capabilities(&self.adapter.physical_device);
                // log::info!("available modes: {:?}", caps.present_modes);

                let mut swapchain_config = SwapchainConfig::from_caps(
                    &caps,
                    self.surface_format.clone().convert(),
                    self.surface_extent.read().clone().convert(),
                );
                // This seems to fix some fullscreen slowdown on macOS.
                if caps.image_count.contains(&3) {
                    swapchain_config.image_count = 3;
                }

                // log::info!("swapchain mode: {:?}", swapchain_config.present_mode);
                swapchain_config.present_mode = PresentMode::IMMEDIATE;

                unsafe { surface.configure_swapchain(&self.device, swapchain_config)? };
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

            match self.surface.lock().acquire_image(acquire_timeout_ns) {
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

    fn add_node(&mut self, node: Node<Self>) {
        self.nodes.insert(builder::build_node(
            self.device.deref(),
            node,
            &self.attachments,
            self.surface_format.clone(),
        ));
    }

    fn add_attachment(&mut self, attachment: GraphAttachment) -> Self::AttachmentIndex {
        let index = self.attachments.insert(attachment);
        AttachmentIndex::Custom(index)
    }

    fn attachment_index(&self, name: Cow<'static, str>) -> Option<Self::AttachmentIndex> {
        self.attachments
            .iter()
            .find(|(_i, a)| a.name == name)
            .map(|(i, _a)| AttachmentIndex::Custom(i))
    }

    fn get_backbuffer_attachment(&self) -> Self::AttachmentIndex {
        AttachmentIndex::Backbuffer
    }

    fn execute(&mut self, world: &World, resources: &Resources) {
        {
            // Check for resize events
            let resize_events = resources
                .get::<app::event::Events<window::events::WindowResize>>()
                .expect("[GfxGraph] (execute) failed to get resize event");

            // Get the last window extent
            if let Some(&window::events::WindowResize(new_extent)) = resize_events.iter().last() {
                *self.surface_extent.write() = Extent2D {
                    width: new_extent.width,
                    height: new_extent.height,
                };
                self.should_configure_swapchain
                    .store(true, Ordering::Relaxed);
            }
        }

        self.configure_swapchain()
            .expect("[GfxGraph] failed to configure swapchain");

        let (index, image) = match self.new_frame() {
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
            let mut command = unsafe { self.command_pool.lock().allocate_one(Level::Primary) };

            // Start the command buffer
            unsafe {
                command.begin_primary(CommandBufferFlags::ONE_TIME_SUBMIT);
            }

            // Command Encoder Abstraction
            let mut gfx_command = GfxCommand::<B>::new(command);
            let extent = self.surface_extent.read().clone();
            let extent3d = Extent {
                width: extent.width,
                height: extent.height,
                depth: 1,
            };

            // TODO: Execute all passes
            for (_node_index, node) in self.nodes.iter_mut() {
                // TODO: Expand to diffrent nodes
                let GfxNode::PassNode(node) = node;
                let framebuffer = unsafe {
                    let mut attachments = node.graph_node.output_attachments.clone();
                    attachments.extend(node.graph_node.input_attachments.clone());
                    if let Some(a) = &node.graph_node.depth_attachment {
                        attachments.push(a.clone())
                    };
                    let attachments = attachments.iter().map(|a| match a.index {
                        AttachmentIndex::Backbuffer => image.borrow(),
                        AttachmentIndex::Custom(_) => {
                            unimplemented!()
                        }
                    });
                    self.device
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
                    vec![Clear::Color(0.2, 0.5, 0.1, 1.0)],
                );

                // Execute Callback
                let frame_data = FrameData {
                    cmd: &mut gfx_command,
                    frame_index: index,
                    viewport: viewport.clone(),
                };
                node.graph_node.callbacks.run(frame_data, world, resources);
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
        let mut graphics_queue = self.queues.graphics.lock();

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
            // NOTE(luca): Currently we do not think about suboptimal swapchains
            let result = graphics_queue.present(
                &mut self.surface.lock(),
                image,
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

    fn get_surface_format(&self) -> TextureFormat {
        self.surface_format.clone()
    }

    fn get_swapchain_image_count(&self) -> usize {
        self.frames_in_flight as usize
    }

    fn build_pass_node<U: render::graph::nodes::callbacks::UserData>(
        &self,
        name: Cow<'static, str>,
    ) -> render::graph::nodes::pass::PassNodeBuilder<Self, U> {
        render::graph::nodes::pass::PassNodeBuilder::new(name)
    }
}

// TODO: Drop the custom nodes
