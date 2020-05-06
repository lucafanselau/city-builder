// Here lives our Main Renderer separated into smaller parts, as I see fit

use log::*;

use winit::dpi::PhysicalSize;

use std::{error::Error, mem::ManuallyDrop};

use gfx_hal::{
    device::Device,
    format::Format,
    queue::family::QueueGroup,
    queue::QueueFamily,
    window::{Extent2D, Surface},
    Backend, Instance,
};

type InitError = Box<dyn Error>;
type RenderError = Box<dyn Error>;

fn compile_shader(
    glsl: &str,
    shader_type: shaderc::ShaderKind,
    shader_name: Option<&str>,
) -> Result<Vec<u32>, InitError> {
    use shaderc::*;
    use std::io::Cursor;

    // for now we will create the compiler inplace
    // should probably be shared between compilations
    let mut compiler = Compiler::new().ok_or("failed to create shaderc compiler")?;
    // let mut options = shaderc::CompileOptions::new().ok_or("failed to create compile options")?;
    let binary_result: shaderc::CompilationArtifact = compiler.compile_into_spirv(
        glsl,
        shader_type,
        shader_name.unwrap_or("shader.glsl"),
        "main",
        None,
    )?;

    let spirv = gfx_hal::pso::read_spirv(Cursor::new(binary_result.as_binary_u8().to_vec()))?;

    Ok(spirv)
}

#[derive(Debug)]
pub struct Renderer<B: Backend> {
    instance: B::Instance,
    surface: ManuallyDrop<B::Surface>,
    adapter: gfx_hal::adapter::Adapter<B>,
    device: B::Device,
    queue_group: QueueGroup<B>,
    surface_format: Format,
    render_pass: ManuallyDrop<B::RenderPass>,
    pipeline_layout: ManuallyDrop<B::PipelineLayout>,
    pipeline: ManuallyDrop<B::GraphicsPipeline>,
    command_pools: Vec<B::CommandPool>,
    command_buffers: Vec<B::CommandBuffer>,
    submission_complete_fences: Vec<B::Fence>,
    rendering_complete_semaphores: Vec<B::Semaphore>,
    frame: u64,
    frames_in_flight: u8,
    should_configure_swapchain: bool,
    surface_extent: Extent2D,
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

        let (device, queue_group): (B::Device, QueueGroup<B>) = {
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

            (B::Device::from(gpu.device), gpu.queue_groups.pop().unwrap())
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

        let render_pass = {
            use gfx_hal::image::Layout;
            use gfx_hal::pass::*;

            let color_attachment = Attachment {
                format: Some(surface_color_format),
                samples: 1,
                ops: AttachmentOps::new(AttachmentLoadOp::Clear, AttachmentStoreOp::Store),
                stencil_ops: AttachmentOps::DONT_CARE,
                layouts: Layout::Undefined..Layout::Present,
            };

            let subpass = SubpassDesc {
                colors: &[(0, Layout::ColorAttachmentOptimal)],
                depth_stencil: None,
                inputs: &[],
                resolves: &[],
                preserves: &[],
            };

            unsafe {
                let render_pass =
                    device.create_render_pass(&[color_attachment], &[subpass], &[])?;
                Ok::<B::RenderPass, InitError>(render_pass)
            }?
        };

        let pipeline_layout = unsafe {
            let pipeline_layout = device.create_pipeline_layout(&[], &[])?;
            Ok::<B::PipelineLayout, InitError>(pipeline_layout)
        }?;

        let pipeline = unsafe {
            use gfx_hal::pass::Subpass;
            use gfx_hal::pso::{
                BlendState, ColorBlendDesc, ColorMask, EntryPoint, Face, GraphicsPipelineDesc,
                GraphicsShaderSet, Primitive, Rasterizer, Specialization,
            };

            let vertex_shader = include_str!("../shaders/triangle.vert");
            let fragment_shader = include_str!("../shaders/triangle.frag");

            let vertex_shader_module = device.create_shader_module(&compile_shader(
                vertex_shader,
                shaderc::ShaderKind::Vertex,
                Some("triangle.vert"),
            )?)?;

            let fragment_shader_module = device.create_shader_module(&compile_shader(
                fragment_shader,
                shaderc::ShaderKind::Fragment,
                Some("triangle.frag"),
            )?)?;

            let (vs_entry, fs_entry) = (
                EntryPoint {
                    entry: "main",
                    module: &vertex_shader_module,
                    specialization: Specialization::default(),
                },
                EntryPoint {
                    entry: "main",
                    module: &fragment_shader_module,
                    specialization: Specialization::default(),
                },
            );

            let shader_set = GraphicsShaderSet {
                vertex: vs_entry,
                hull: None,
                domain: None,
                geometry: None,
                fragment: Some(fs_entry),
            };

            let mut pipeline_desc = GraphicsPipelineDesc::new(
                shader_set,
                Primitive::TriangleList,
                Rasterizer {
                    cull_face: Face::BACK,
                    ..Rasterizer::FILL
                },
                &pipeline_layout,
                Subpass {
                    index: 0,
                    main_pass: &render_pass,
                },
            );

            pipeline_desc.blender.targets.push(ColorBlendDesc {
                mask: ColorMask::ALL,
                blend: Some(BlendState::ALPHA),
            });

            let pipeline = device.create_graphics_pipeline(&pipeline_desc, None)?;

            device.destroy_shader_module(vertex_shader_module);
            device.destroy_shader_module(fragment_shader_module);

            Ok::<B::GraphicsPipeline, InitError>(pipeline)
        }?;

        let physical_size = window.inner_size();
        let surface_extent = Extent2D {
            width: physical_size.width,
            height: physical_size.height,
        };

        Ok(Renderer {
            instance,
            surface: ManuallyDrop::new(surface),
            adapter,
            device,
            queue_group,
            surface_format: surface_color_format,
            render_pass: ManuallyDrop::new(render_pass),
            pipeline_layout: ManuallyDrop::new(pipeline_layout),
            pipeline: ManuallyDrop::new(pipeline),
            command_pools,
            command_buffers,
            submission_complete_fences,
            rendering_complete_semaphores,
            frame: 0,
            frames_in_flight,
            surface_extent,
            should_configure_swapchain: true,
        })
    }

		fn spawn_watcher_thread(&self) {

				// let device = &self.device;
				// let fence = &self.submission_complete_fences[0];
				
				// let handle = std::thread::spawn(move || {
				// 		device.wait_for_fence(fence, 1_000_000_000);
				// });
																			
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

    pub fn render(&mut self) -> Result<(), RenderError> {
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
                &self.render_pass,
                vec![surface_image.borrow()],
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

        unsafe {
            use gfx_hal::command::{
                ClearColor, ClearValue, CommandBuffer, CommandBufferFlags, SubpassContents,
            };

            let cmd: &mut B::CommandBuffer = &mut self.command_buffers[frame_idx];

            cmd.begin_primary(CommandBufferFlags::ONE_TIME_SUBMIT);

            cmd.set_viewports(0, &[viewport.clone()]);
            cmd.set_scissors(0, &[viewport.rect]);

            cmd.begin_render_pass(
                &self.render_pass,
                &framebuffer,
                viewport.rect,
                &[ClearValue {
                    color: ClearColor {
                        float32: [0.0, 0.0, 0.0, 1.0],
                    },
                }],
                SubpassContents::Inline,
            );

            cmd.bind_graphics_pipeline(&self.pipeline);

            cmd.draw(0..3, 0..1);

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

            self.device
                .destroy_graphics_pipeline(ManuallyDrop::take(&mut self.pipeline));
            self.device
                .destroy_pipeline_layout(ManuallyDrop::take(&mut self.pipeline_layout));

            self.device
                .destroy_render_pass(ManuallyDrop::take(&mut self.render_pass));

            for cmd_pool in self.command_pools.drain(..) {
                self.device.destroy_command_pool(cmd_pool)
            }

            self.surface.unconfigure_swapchain(&self.device);
            self.instance
                .destroy_surface(ManuallyDrop::take(&mut self.surface));
        }
    }
}
