// Here lives our Main Renderer separated into smaller parts, as I see fit
mod shaders;
use shaders::Pipeline;
mod vertex;

use log::*;

use winit::dpi::PhysicalSize;

use std::{error::Error, mem::ManuallyDrop, rc::Rc, sync::Arc, time::Instant};

use nalgebra_glm as glm;

use gfx_hal::{
    device::Device,
    format::Format,
    queue::family::QueueGroup,
    queue::QueueFamily,
    window::{Extent2D, Surface},
    Backend, Instance,
};

use crate::camera;

type InitError = Box<dyn Error>;
type RenderError = Box<dyn Error>;

// This is data, that is not allowed to be destroyed before the frame finished
#[derive(Debug, Clone)]
pub struct FrameData<B: Backend> {
    pipeline: Rc<Pipeline<B>>,
}

#[derive(Debug)]
pub struct RenderPass<B: Backend> {
    render_pass: ManuallyDrop<B::RenderPass>,
    device: Arc<B::Device>,
}

impl<B: Backend> Drop for RenderPass<B> {
    fn drop(&mut self) {
        unsafe {
            self.device
                .destroy_render_pass(ManuallyDrop::take(&mut self.render_pass));
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PushConstants {
    // transform: [[f32; 4]; 4],
    transform: glm::Mat4,
    // view_projection: [[f32; 4]; 4],
    view_projection: glm::Mat4,
}

unsafe fn push_constant_bytes<T>(push_constants: &T) -> &[u32] {
    let size_in_bytes = std::mem::size_of::<T>();
    let size_in_u32s = size_in_bytes / std::mem::size_of::<u32>();
    let start_ptr = push_constants as *const T as *const u32;
    std::slice::from_raw_parts(start_ptr, size_in_u32s)
}

#[derive(Debug)]
pub struct Renderer<B: Backend>
where
    B::Device: Send + Sync,
{
    instance: B::Instance,
    surface: ManuallyDrop<B::Surface>,
    adapter: gfx_hal::adapter::Adapter<B>,
    device: Arc<B::Device>,
    queue_group: QueueGroup<B>,
    surface_format: Format,
    render_pass: Arc<RenderPass<B>>,
    shader_system: shaders::ShaderSystem<B>,
    // pipeline_layout: ManuallyDrop<B::PipelineLayout>,
    // pipeline: ManuallyDrop<B::GraphicsPipeline>,
    frame_data: Vec<Option<FrameData<B>>>,
    command_pools: Vec<B::CommandPool>,
    command_buffers: Vec<B::CommandBuffer>,
    submission_complete_fences: Vec<B::Fence>,
    rendering_complete_semaphores: Vec<B::Semaphore>,
    frame: u64,
    frames_in_flight: u8,
    should_configure_swapchain: bool,
    surface_extent: Extent2D,
    mesh: vertex::Mesh<B>,

    // Depth Ressources
    depth_image: Option<B::Image>,
    depth_image_memory: Option<B::Memory>,
    depth_image_view: Option<B::ImageView>,
}

impl<B: Backend> Renderer<B> {
    pub fn new(window: &winit::window::Window) -> Result<Renderer<B>, InitError> {
        let (instance, adapters, surface) = {
            let instance =
                B::Instance::create("Mightycity", 1).expect("failed to create a instance");

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

        // our adapter selection is a bit more sophisticated
        let adapter = adapters
            .into_iter()
            .find(|a| {
                a.queue_families.iter().any(|qf| {
                    qf.queue_type().supports_graphics() && surface.supports_queue_family(qf)
                })
            })
            .expect("couldn't find suitable adapter");
        debug!("Selected: {:?}", adapter.info);

        let (device, queue_group): (Arc<B::Device>, QueueGroup<B>) = {
            // need to find the queue_family
            let queue_family = adapter
                .queue_families
                .iter()
                .find(|family| {
                    surface.supports_queue_family(family) && family.queue_type().supports_graphics()
                })
                .expect("couldn't find queue_family");

            let mut gpu = unsafe {
                use gfx_hal::adapter::PhysicalDevice;

                adapter
                    .physical_device
                    .open(&[(queue_family, &[1.0; 1])], gfx_hal::Features::empty())
                    .expect("failed to open device")
            };

            (Arc::new(gpu.device), gpu.queue_groups.pop().unwrap())
        };

        let frames_in_flight = 3u8;

        let (command_pools, command_buffers) = unsafe {
            use gfx_hal::command::Level;
            use gfx_hal::pool::{CommandPool, CommandPoolCreateFlags};

            let mut command_pools = Vec::with_capacity(frames_in_flight as usize);
            let mut command_buffers = Vec::with_capacity(frames_in_flight as usize);

            for _ in 0..frames_in_flight {
                let mut command_pool: B::CommandPool = device
                    .create_command_pool(queue_group.family, CommandPoolCreateFlags::empty())?;
                command_buffers.push(command_pool.allocate_one(Level::Primary));
                command_pools.push(command_pool);
            }

            Ok::<(Vec<B::CommandPool>, Vec<B::CommandBuffer>), InitError>((
                command_pools,
                command_buffers,
            ))
        }?;

        let mut submission_complete_fences = Vec::with_capacity(frames_in_flight as usize);
        let mut rendering_complete_semaphores = Vec::with_capacity(frames_in_flight as usize);

        for _ in 0..frames_in_flight {
            submission_complete_fences.push(device.create_fence(true)?);
            rendering_complete_semaphores.push(device.create_semaphore()?);
        }

        // From here one render stuff

        let surface_color_format = {
            use gfx_hal::format::ChannelType;

            let supported_formats = surface
                .supported_formats(&adapter.physical_device)
                .unwrap_or(vec![]);

            let default_format = *supported_formats.get(0).unwrap_or(&Format::Rgba8Srgb);

            supported_formats
                .into_iter()
                .find(|format| format.base_format().1 == ChannelType::Srgb)
                .unwrap_or(default_format)
        };

        let render_pass_raw = {
            use gfx_hal::image::Layout;
            use gfx_hal::pass::*;

            let color_attachment = Attachment {
                format: Some(surface_color_format),
                samples: 1,
                ops: AttachmentOps::new(AttachmentLoadOp::Clear, AttachmentStoreOp::Store),
                stencil_ops: AttachmentOps::DONT_CARE,
                layouts: Layout::Undefined..Layout::Present,
            };

            let depth_format = get_depth_format(&adapter);
            info!("Depth Format is: {:#?}", depth_format);
            let depth_attachment = Attachment {
                format: Some(depth_format),
                samples: 1,
                ops: AttachmentOps::new(AttachmentLoadOp::Clear, AttachmentStoreOp::DontCare),
                stencil_ops: AttachmentOps::DONT_CARE,
                layouts: Layout::Undefined..Layout::DepthStencilAttachmentOptimal,
            };

            let subpass = SubpassDesc {
                colors: &[(0, Layout::ColorAttachmentOptimal)],
                depth_stencil: Some(&(1, Layout::DepthStencilAttachmentOptimal)),
                inputs: &[],
                resolves: &[],
                preserves: &[],
            };

            unsafe {
                let render_pass = device.create_render_pass(
                    &[color_attachment, depth_attachment],
                    &[subpass],
                    &[],
                )?;
                Ok::<B::RenderPass, InitError>(render_pass)
            }?
        };

        let render_pass = Arc::new(RenderPass {
            render_pass: ManuallyDrop::new(render_pass_raw),
            device: device.clone(),
        });

        let shader_system = shaders::ShaderSystem::new(device.clone(), render_pass.clone());

        let physical_size = window.inner_size();
        let surface_extent = Extent2D {
            width: physical_size.width,
            height: physical_size.height,
        };

        let frame_data = vec![None; frames_in_flight as usize];

        // load the model
        let mesh = vertex::create_mesh(device.clone(), &adapter)?;

        Ok(Renderer {
            instance,
            surface: ManuallyDrop::new(surface),
            adapter,
            device,
            queue_group,
            surface_format: surface_color_format,
            render_pass,
            shader_system,
            frame_data,
            command_pools,
            command_buffers,
            submission_complete_fences,
            rendering_complete_semaphores,
            frame: 0,
            frames_in_flight,
            should_configure_swapchain: true,
            surface_extent,
            mesh,
            depth_image: None,
            depth_image_memory: None,
            depth_image_view: None,
        })
    }

    /// Should be used to wait before changing the swapchain
    unsafe fn wait_for_fences(&self) -> Result<(), RenderError> {
        use gfx_hal::device::WaitFor;
        let wait_timeout_ns = 1_000_000_000;

        self.device.wait_for_fences(
            &self.submission_complete_fences,
            WaitFor::All,
            wait_timeout_ns,
        )?;

        Ok(())
    }

    unsafe fn create_depth_image(&mut self) -> Result<(), RenderError> {
        use gfx_hal::{
            adapter::PhysicalDevice,
            format::{Aspects, Swizzle},
            image::{Kind, SubresourceRange, Tiling, Usage, ViewCapabilities, ViewKind},
            memory::Properties,
            MemoryTypeId,
        };

        let format = get_depth_format(&self.adapter);

        let kind = Kind::D2(self.surface_extent.width, self.surface_extent.height, 1, 1);

        let mut image = self.device.create_image(
            kind,
            1,
            format,
            Tiling::Optimal,
            Usage::DEPTH_STENCIL_ATTACHMENT,
            ViewCapabilities::empty(),
        )?;

        let requirements = self.device.get_image_requirements(&image);

        let memory_types = self
            .adapter
            .physical_device
            .memory_properties()
            .memory_types;

        let memory_type = memory_types
            .iter()
            .enumerate()
            .find(|(id, mem_type)| {
                let type_supported = requirements.type_mask & (1_u64 << id) != 0;
                type_supported && mem_type.properties.contains(Properties::DEVICE_LOCAL)
            })
            .map(|(id, _ty)| MemoryTypeId(id))
            .expect("did not find memory type");

        let image_memory = self
            .device
            .allocate_memory(memory_type, requirements.size)?;

        {
            self.device
                .bind_image_memory(&image_memory, 0, &mut image)?;
        }

        // Create Image View
        let image_view = self.device.create_image_view(
            &image,
            ViewKind::D2,
            format,
            Swizzle::NO,
            SubresourceRange {
                aspects: Aspects::DEPTH,
                levels: 0..1,
                layers: 0..1,
            },
        )?;

        // Now we can set the fields
        self.depth_image = Some(image);
        self.depth_image_memory = Some(image_memory);
        self.depth_image_view = Some(image_view);

        Ok::<(), RenderError>(())
    }

    fn configure_swapchain(&mut self) -> Result<(), RenderError> {
        if self.should_configure_swapchain {
            use gfx_hal::window::{PresentationSurface, SwapchainConfig};

            // First lets wait for all the frames
            unsafe {
                self.wait_for_fences()?;
                Ok::<(), RenderError>(())
            }?;

            let caps = self.surface.capabilities(&self.adapter.physical_device);

            let mut swapchain_config =
                SwapchainConfig::from_caps(&caps, self.surface_format, self.surface_extent);
            // This seems to fix some fullscreen slowdown on macOS.
            if caps.image_count.contains(&3) {
                swapchain_config.image_count = 3;
            }

            self.surface_extent = swapchain_config.extent;

            unsafe {
                self.surface
                    .configure_swapchain(&self.device, swapchain_config)?;
                Ok::<(), RenderError>(())
            }?;

            unsafe {
                self.create_depth_image()?;
                Ok::<(), RenderError>(())
            }?;

            self.should_configure_swapchain = false;
        }

        Ok(())
    }

    /// Handle Window Event Resize & ScaleFactorChanged
    pub fn handle_resize(&mut self, d: PhysicalSize<u32>) {
        self.surface_extent = Extent2D {
            width: d.width,
            height: d.height,
        };
        self.should_configure_swapchain = true;
    }

    pub fn render(&mut self, start_time: &Instant, camera: &camera::Camera) -> Result<(), RenderError> {
        // The index for the in flight ressources
        let frame_idx: usize = self.frame as usize % self.frames_in_flight as usize;

        // See if we need to recreate the swapchain
        self.configure_swapchain()?;

        // Wait for the in flight image of the current index
        unsafe {
            use gfx_hal::pool::CommandPool;

            let render_timeout_ns = 1_000_000_000;

            self.device.wait_for_fence(
                &self.submission_complete_fences[frame_idx],
                render_timeout_ns,
            )?;

            self.device
                .reset_fence(&self.submission_complete_fences[frame_idx])?;

            // ok so this frame is ready -> We can create a Frame Data Object
            self.frame_data[frame_idx] = Some(FrameData {
                pipeline: self.shader_system.get_pipeline(),
            });

            self.command_pools[frame_idx].reset(false);

            Ok::<(), RenderError>(())
        }?;

        let surface_image = unsafe {
            use gfx_hal::window::PresentationSurface;
            // We refuse to wait more than a second, to avoid hanging.
            let acquire_timeout_ns = 1_000_000_000;

            match self.surface.acquire_image(acquire_timeout_ns) {
                Ok((image, _)) => image,
                Err(e) => {
                    self.should_configure_swapchain = true;
                    return Err(Box::new(e));
                }
            }
        };

        let framebuffer = unsafe {
            use std::borrow::Borrow;

            use gfx_hal::image::Extent;

            let framebuffer = self.device.create_framebuffer(
                &self.render_pass.render_pass,
                vec![
                    surface_image.borrow(),
                    self.depth_image_view
                        .as_ref()
                        .expect("depth image view missing"),
                ],
                Extent {
                    width: self.surface_extent.width,
                    height: self.surface_extent.height,
                    depth: 1,
                },
            )?;

            Ok::<B::Framebuffer, RenderError>(framebuffer)
        }?;

        let viewport = {
            use gfx_hal::pso::{Rect, Viewport};

            Viewport {
                rect: Rect {
                    x: 0,
                    y: 0,
                    w: self.surface_extent.width as i16,
                    h: self.surface_extent.height as i16,
                },
                depth: 0.0..1.0,
            }
        };

        let angle = start_time.elapsed().as_secs_f32();

        let teapots = {
            let transform = glm::rotate(&glm::Mat4::identity(), angle, &glm::vec3(0., 1., 0.));

            &[PushConstants {
                transform,
                view_projection: camera.data.expect("You need to call camera.update(dt)"),
            }]
        };

        unsafe {
            use gfx_hal::command::{
                ClearColor, ClearDepthStencil, ClearValue, CommandBuffer, CommandBufferFlags,
                SubpassContents,
            };

            let cmd: &mut B::CommandBuffer = &mut self.command_buffers[frame_idx];

            cmd.begin_primary(CommandBufferFlags::ONE_TIME_SUBMIT);
						
            cmd.set_viewports(0, &[viewport.clone()]);
            cmd.set_scissors(0, &[viewport.rect]);

            cmd.begin_render_pass(
                &self.render_pass.render_pass,
                &framebuffer,
                viewport.rect,
                &[
                    ClearValue {
                        color: ClearColor {
                            float32: [0.0, 0.0, 0.0, 1.0],
                        },
                    },
                    ClearValue {
                        depth_stencil: ClearDepthStencil {
                            depth: 1.,
                            stencil: 0,
                        },
                    },
                ],
                SubpassContents::Inline,
            );

            cmd.bind_graphics_pipeline(
                &self.frame_data[frame_idx]
                    .as_ref()
                    .unwrap()
                    .pipeline
                    .pipeline,
            );

            cmd.bind_vertex_buffers(
                0,
                vec![(
                    &self.mesh.vertex_buffer as &B::Buffer,
                    gfx_hal::buffer::SubRange::WHOLE,
                )],
            );

            for teapot in teapots {
                use gfx_hal::pso::ShaderStageFlags;

                cmd.push_graphics_constants(
                    &self.frame_data[frame_idx]
                        .as_ref()
                        .unwrap()
                        .pipeline
                        .pipeline_layout,
                    ShaderStageFlags::VERTEX,
                    0,
                    push_constant_bytes(teapot),
                );

                cmd.draw(0..self.mesh.vertex_length, 0..1);
            }

            cmd.end_render_pass();
            cmd.finish();
        };

        unsafe {
            use gfx_hal::queue::{CommandQueue, Submission};
            use std::borrow::Borrow;

            let semaphore: &B::Semaphore = self.rendering_complete_semaphores[frame_idx].borrow();

            let submission = Submission {
                command_buffers: vec![&self.command_buffers[frame_idx]],
                wait_semaphores: None,
                signal_semaphores: vec![semaphore],
            };

            self.queue_group.queues[0].submit(
                submission,
                Some(&self.submission_complete_fences[frame_idx]),
            );
            let result = self.queue_group.queues[0].present_surface(
                &mut self.surface,
                surface_image,
                Some(&self.rendering_complete_semaphores[frame_idx]),
            );

            self.should_configure_swapchain |= result.is_err();

            self.device.destroy_framebuffer(framebuffer);
        }

        self.frame += 1;

        Ok::<(), RenderError>(())
    }
}

impl<B: Backend> Drop for Renderer<B> {
    fn drop(&mut self) {
        use gfx_hal::window::PresentationSurface;

        self.device.wait_idle().expect("failed to wait idle");
        unsafe {
            for semaphore in self.rendering_complete_semaphores.drain(..) {
                self.device.destroy_semaphore(semaphore);
            }

            for fence in self.submission_complete_fences.drain(..) {
                self.device.destroy_fence(fence);
            }

            // self.device
            //     .destroy_graphics_pipeline(ManuallyDrop::take(&mut self.pipeline));
            // self.device
            //     .destroy_pipeline_layout(ManuallyDrop::take(&mut self.pipeline_layout));

            for cmd_pool in self.command_pools.drain(..) {
                self.device.destroy_command_pool(cmd_pool)
            }

            // Destroy Depth Fields
            if let Some(image) = self.depth_image.take() {
                self.device.destroy_image(image);
            }

            if let Some(image_view) = self.depth_image_view.take() {
                self.device.destroy_image_view(image_view);
            }

            if let Some(image_memory) = self.depth_image_memory.take() {
                self.device.free_memory(image_memory);
            }

            self.surface.unconfigure_swapchain(&self.device);
            self.instance
                .destroy_surface(ManuallyDrop::take(&mut self.surface));
        }
    }
}

/// UTILITY FUNCTIONS
fn get_depth_format<B: Backend>(adapter: &gfx_hal::adapter::Adapter<B>) -> gfx_hal::format::Format {
    use gfx_hal::{adapter::PhysicalDevice, format::ImageFeature, image::Tiling};

    let candidates = [
        Format::D32Sfloat,
        Format::D32SfloatS8Uint,
        Format::D24UnormS8Uint,
    ];

    let tiling = Tiling::Optimal;

    let image_features = ImageFeature::DEPTH_STENCIL_ATTACHMENT;

    let mut result_format = None;

    'search_loop: for candidate in &candidates {
        let properties = adapter
            .physical_device
            .format_properties(Some(candidate.clone()));

        match tiling {
            Tiling::Optimal => {
                if (properties.optimal_tiling & image_features) == image_features {
                    result_format = Some(candidate.clone());
                    break 'search_loop;
                }
            }
            Tiling::Linear => {
                if (properties.linear_tiling & image_features) == image_features {
                    result_format = Some(candidate.clone());
                    break 'search_loop;
                }
            }
        }
    }

    result_format.expect("failed to find format")
}
