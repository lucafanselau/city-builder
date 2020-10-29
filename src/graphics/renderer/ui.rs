use imgui::{Context, DrawCmd, DrawCmdParams, DrawData, DrawIdx, DrawVert, ImString, Ui};
use imgui_winit_support::{HiDpiMode, WinitPlatform};

use log::*;

use winit::{event::Event, window::Window};

use gfx_hal::{Backend, IndexType};

use crate::renderer::shaders::{ConstructData, DescriptorLayout, Pipeline, ShaderSystem};
use crate::renderer::vertex::find_memory_type;
use crate::renderer::{push_constant_bytes, shaders, vertex};
use gfx_hal::buffer::IndexBufferView;
use gfx_hal::command::CommandBuffer;
use gfx_hal::device::Device;
use gfx_hal::pool::CommandPool;
use gfx_hal::pso::{
    AttributeDesc, DescriptorPool, Element, Rect, VertexBufferDesc, VertexInputRate,
};
use gfx_hal::queue::CommandQueue;
use nalgebra_glm as glm;
use std::borrow::Borrow;
use std::mem::ManuallyDrop;
use std::{rc::Rc, sync::Arc, time::Instant};

const VTX_COUNT: usize = 8_192;
const IDX_COUNT: usize = 8_192;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UIPushConstants {
    matrix: glm::Mat4,
}

#[derive(Debug)]
struct FontAtlasTexture<B: Backend> {
    image: ManuallyDrop<B::Image>,
    memory: ManuallyDrop<B::Memory>,
    image_view: ManuallyDrop<B::ImageView>,
    sampler: ManuallyDrop<B::Sampler>,
}

impl<B: Backend> FontAtlasTexture<B> {
    pub fn create(
        device: &Arc<B::Device>,
        adapter: &gfx_hal::adapter::Adapter<B>,
        imgui: &mut Context,
        cmd_pool: &mut B::CommandPool,
        cmd_queue: &mut B::CommandQueue,
    ) -> Self {
        let mut fonts = imgui.fonts();
        let texture = fonts.build_rgba32_texture();

        // 0. First we compute some memory related values.
        let pixel_size = 4; // By definition
                            // let row_size = pixel_size * (texture.width as usize);
                            // let limits = adapter.physical_device.limits();
                            // let row_alignment_mask = limits.optimal_buffer_copy_pitch_alignment as u32 - 1;
                            // let row_pitch = ((row_size as u32 + row_alignment_mask) & !row_alignment_mask) as usize;
                            // debug_assert!(row_pitch as usize >= row_size);

        // Create Staging Buffer
        let required_bytes = (texture.width * texture.height * pixel_size) as usize;
        let staging_buffer = unsafe {
            use gfx_hal::{buffer::Usage, memory::Properties};
            vertex::make_buffer::<B>(
                device.as_ref(),
                &adapter.physical_device,
                required_bytes,
                Usage::TRANSFER_SRC,
                Properties::CPU_VISIBLE | Properties::COHERENT,
            )
            .expect("failed to create staging buffer for ui texture atlas")
        };

        // Write to that Buffer
        unsafe {
            use gfx_hal::memory::Segment;

            let mapped_memory = device
                .map_memory(&staging_buffer.0, Segment::ALL)
                .expect("failed to acquire staging memory pointer");

            std::ptr::copy_nonoverlapping(texture.data.as_ptr(), mapped_memory, required_bytes);

            device
                .flush_mapped_memory_ranges(vec![(&staging_buffer.0, Segment::ALL)])
                .expect("TODO");

            device.unmap_memory(&staging_buffer.0);
        };

        // create image
        let (image, memory) = unsafe {
            use gfx_hal::{
                format::Format,
                image::{Kind, Tiling, Usage, ViewCapabilities},
                memory::Properties,
            };

            let mut image = device
                .create_image(
                    Kind::D2(texture.width, texture.height, 1, 1),
                    1,
                    Format::Rgba8Srgb,
                    Tiling::Optimal,
                    Usage::TRANSFER_DST | Usage::SAMPLED,
                    ViewCapabilities::empty(),
                )
                .expect("failed to create ui font texture image");

            let requirements = device.get_image_requirements(&image);

            let memory_type = find_memory_type::<B>(
                &adapter.physical_device,
                &requirements,
                Properties::DEVICE_LOCAL,
            );

            let image_memory = device
                .allocate_memory(memory_type, requirements.size)
                .expect("failed to create ui font texture memory");

            device
                .bind_image_memory(&image_memory, 0, &mut image)
                .expect("failed to bind image memory");

            (image, image_memory)
        };

        // Copy over the data
        unsafe {
            use gfx_hal::{
                command::{BufferImageCopy, CommandBufferFlags, Level},
                format::Aspects,
                image::{Access, Extent, Layout, Offset, SubresourceLayers, SubresourceRange},
                memory::Barrier,
                pso::PipelineStage,
            };

            let mut cmd = cmd_pool.allocate_one(Level::Primary);
            cmd.begin_primary(CommandBufferFlags::ONE_TIME_SUBMIT);

            let prepare_image_barrier = Barrier::Image {
                states: (Access::empty(), Layout::Undefined)
                    ..(Access::TRANSFER_WRITE, Layout::TransferDstOptimal),
                target: &image,
                families: None,
                range: SubresourceRange {
                    aspects: Aspects::COLOR,
                    levels: 0..1,
                    layers: 0..1,
                },
            };
            cmd.pipeline_barrier(
                PipelineStage::TOP_OF_PIPE..PipelineStage::TRANSFER,
                gfx_hal::memory::Dependencies::empty(),
                &[prepare_image_barrier],
            );

            cmd.copy_buffer_to_image(
                &staging_buffer.1,
                &image,
                Layout::TransferDstOptimal,
                &[BufferImageCopy {
                    buffer_offset: 0,
                    buffer_width: texture.width,
                    buffer_height: texture.height,
                    image_layers: SubresourceLayers {
                        aspects: Aspects::COLOR,
                        level: 0,
                        layers: 0..1,
                    },
                    image_offset: Offset { x: 0, y: 0, z: 0 },
                    image_extent: Extent {
                        width: texture.width,
                        height: texture.height,
                        depth: 1,
                    },
                }],
            );

            let finish_image_barrier = Barrier::Image {
                states: (Access::TRANSFER_WRITE, Layout::TransferDstOptimal)
                    ..(Access::SHADER_READ, Layout::ShaderReadOnlyOptimal),
                target: &image,
                families: None,
                range: SubresourceRange {
                    aspects: Aspects::COLOR,
                    levels: 0..1,
                    layers: 0..1,
                },
            };
            cmd.pipeline_barrier(
                PipelineStage::TRANSFER..PipelineStage::FRAGMENT_SHADER,
                gfx_hal::memory::Dependencies::empty(),
                &[finish_image_barrier],
            );

            cmd.finish();
            let upload_fence = device
                .create_fence(false)
                .expect("failed to create upload fence");

            cmd_queue.submit_without_semaphores(&[&cmd], Some(&upload_fence));
            device
                .wait_for_fence(&upload_fence, core::u64::MAX)
                .expect("failed to wait for cmd buffer");
            device.destroy_fence(upload_fence);
            cmd_pool.free(vec![cmd]);
        };

        // Destroy Staging Buffer
        unsafe {
            device.destroy_buffer(staging_buffer.1);
            device.free_memory(staging_buffer.0);
        };

        // create image view and sampler
        let (image_view, sampler) = unsafe {
            use gfx_hal::{
                format::{Aspects, Format, Swizzle},
                image::{Filter, SamplerDesc, SubresourceRange, ViewKind, WrapMode},
            };

            let image_view = device
                .create_image_view(
                    &image,
                    ViewKind::D2,
                    Format::Rgba8Srgb,
                    Swizzle::NO,
                    SubresourceRange {
                        aspects: Aspects::COLOR,
                        levels: 0..1,
                        layers: 0..1,
                    },
                )
                .expect("failed to create font atlas image");
            let sampler = device
                .create_sampler(&SamplerDesc::new(Filter::Nearest, WrapMode::Tile))
                .expect("failed to create font atlas sampler");

            (image_view, sampler)
        };
        // Okay we should now have a correctly filled image
        FontAtlasTexture {
            image: ManuallyDrop::new(image),
            memory: ManuallyDrop::new(memory),
            image_view: ManuallyDrop::new(image_view),
            sampler: ManuallyDrop::new(sampler),
        }
    }

    unsafe fn free(&mut self, device: &B::Device) {
        device.destroy_sampler(ManuallyDrop::take(&mut self.sampler));
        device.destroy_image_view(ManuallyDrop::take(&mut self.image_view));
        device.destroy_image(ManuallyDrop::take(&mut self.image));
        device.free_memory(ManuallyDrop::take(&mut self.memory));
    }
}

#[derive(Debug)]
struct DrawInfo {
    clip_rect: [f32; 4],
    count: usize,
    vertex_offset: i32,
    index_offset: u32,
}

#[derive(Debug)]
struct FrameData<B: Backend> {
    vertex_buffer: (B::Memory, B::Buffer),
    index_buffer: (B::Memory, B::Buffer),
    pipeline: Option<Rc<Pipeline<B>>>,
    matrix: glm::Mat4,
    draw_infos: Vec<DrawInfo>,
    fb_width: f32,
    fb_height: f32,
    clip_off: [f32; 2],
    clip_scale: [f32; 2],
}

pub struct UiHandle {
    imgui: Context,
    pub ui: Ui<'static>,
}

impl UiHandle {
    pub fn new(mut imgui: Context) -> UiHandle {
        let ctx_ptr = &mut imgui as *mut Context;
        let ui = unsafe { &mut *ctx_ptr }.frame();

        UiHandle { imgui, ui }
    }
}

#[derive(Debug)]
pub struct ImGuiRenderer<B: Backend>
where
    B::Device: Send + Sync,
{
    platform: WinitPlatform,
    imgui: Option<Context>,
    last_frame: Instant,
    window: Rc<Window>,
    device: Arc<B::Device>,
    frames: Vec<FrameData<B>>,
    font_atlas: FontAtlasTexture<B>,
    layout: Arc<DescriptorLayout<B>>,
    descriptor_pool: ManuallyDrop<B::DescriptorPool>,
    descriptor_set: B::DescriptorSet,
}

impl<B: Backend> ImGuiRenderer<B> {
    pub fn new(
        window: Rc<Window>,
        device: Arc<B::Device>,
        shader_system: &mut shaders::ShaderSystem<B>,
        adapter: &gfx_hal::adapter::Adapter<B>,
        cmd_pool: &mut B::CommandPool,
        cmd_queue: &mut B::CommandQueue,
        frames_in_flight: u8,
    ) -> Result<ImGuiRenderer<B>, Box<dyn std::error::Error>> {
        let mut imgui = Context::create();

        let mut platform = WinitPlatform::init(&mut imgui);
        platform.attach_window(imgui.io_mut(), &window, HiDpiMode::Default);

        {
            let hidpi_factor = platform.hidpi_factor();
            info!("HIGH DPI Factor: {}", hidpi_factor);
            imgui.fonts().add_font(&[imgui::FontSource::TtfData {
                data: include_bytes!("../../../assets/fonts/FiraCode-Regular.ttf"),
                size_pixels: (13.0 * hidpi_factor) as f32,
                config: None,
            }]);
        }

        imgui.set_renderer_name(Some(ImString::new("citybuilder-imgui-renderer")));

        let vertex_size = std::mem::size_of::<DrawVert>() as u32;
        // Renderer Setup
        // We need a Pipeline, Buffers, Font Texture and
        // The Descriptor Set Layout
        let description_set_layout = Arc::new(DescriptorLayout {
            device: device.clone(),
            layout: ManuallyDrop::new(unsafe {
                use gfx_hal::pso::{
                    DescriptorSetLayoutBinding, DescriptorType, ImageDescriptorType,
                    ShaderStageFlags,
                };
                device
                    .create_descriptor_set_layout(
                        &[
                            DescriptorSetLayoutBinding {
                                binding: 0,
                                ty: DescriptorType::Image {
                                    ty: ImageDescriptorType::Sampled {
                                        with_sampler: false,
                                    },
                                },
                                count: 1,
                                stage_flags: ShaderStageFlags::FRAGMENT,
                                immutable_samplers: false,
                            },
                            DescriptorSetLayoutBinding {
                                binding: 1,
                                ty: DescriptorType::Sampler,
                                count: 1,
                                stage_flags: ShaderStageFlags::FRAGMENT,
                                immutable_samplers: false,
                            },
                        ],
                        &[],
                    )
                    .expect("failed to create descriptor set layout")
            }),
        });

        // The Pipeline
        {
            use gfx_hal::format::Format;
            use gfx_hal::pso::ShaderStageFlags;

            let push_constant_bytes = std::mem::size_of::<UIPushConstants>() as u32;

            let vertex_buffers = vec![VertexBufferDesc {
                binding: 0,
                stride: vertex_size,
                rate: VertexInputRate::Vertex,
            }];

            let attributes = vec![
                // pos
                AttributeDesc {
                    location: 0,
                    binding: 0,
                    element: Element {
                        format: Format::Rg32Sfloat,
                        offset: 0,
                    },
                },
                // UV
                AttributeDesc {
                    location: 1,
                    binding: 0,
                    element: Element {
                        format: Format::Rg32Sfloat,
                        offset: 8, // Hardcode!
                    },
                },
                // Color
                AttributeDesc {
                    location: 2,
                    binding: 0,
                    element: Element {
                        format: Format::Rgba8Unorm,
                        offset: 16, // Hardcode!
                    },
                },
            ];

            let ui_construct_data = ConstructData::new(
                "ui.vert".to_string(),
                "ui.frag".to_string(),
                vertex_buffers,
                attributes,
                false,
                gfx_hal::pso::Face::NONE,
                vec![(ShaderStageFlags::VERTEX, 0..push_constant_bytes)],
                vec![description_set_layout.clone()],
            );
            shader_system.add_pipeline("ui".to_string(), ui_construct_data);
        }

        let mut descriptor_pool = unsafe {
            use gfx_hal::pso::{
                DescriptorPoolCreateFlags, DescriptorRangeDesc, DescriptorType, ImageDescriptorType,
            };
            device
                .create_descriptor_pool(
                    1,
                    &[
                        DescriptorRangeDesc {
                            ty: DescriptorType::Image {
                                ty: ImageDescriptorType::Sampled {
                                    with_sampler: false,
                                },
                            },
                            count: 1,
                        },
                        DescriptorRangeDesc {
                            ty: DescriptorType::Sampler,
                            count: 1,
                        },
                    ],
                    DescriptorPoolCreateFlags::empty(),
                )
                .expect("failed to create descriptor pool")
        };

        let descriptor_set = unsafe {
            descriptor_pool
                .allocate_set(description_set_layout.layout.borrow())
                .expect("failed to allocate descriptor set")
        };

        let font_atlas =
            { FontAtlasTexture::create(&device, adapter, &mut imgui, cmd_pool, cmd_queue) };

        // Write to that descriptor set
        unsafe {
            use gfx_hal::{
                image::Layout,
                pso::{Descriptor, DescriptorSetWrite},
            };
            use std::ops::Deref;

            device.write_descriptor_sets(vec![
                DescriptorSetWrite {
                    set: &descriptor_set,
                    binding: 0,
                    array_offset: 0,
                    descriptors: Some(Descriptor::Image(
                        font_atlas.image_view.deref(),
                        Layout::ShaderReadOnlyOptimal,
                    )),
                },
                DescriptorSetWrite {
                    set: &descriptor_set,
                    binding: 1,
                    array_offset: 0,
                    descriptors: Some(Descriptor::Sampler(font_atlas.sampler.deref())),
                },
            ]);
        }

        let mut frames = Vec::new();

        frames.reserve_exact(frames_in_flight as usize);
        for _ in 0..frames_in_flight {
            unsafe {
                use gfx_hal::{buffer::Usage, memory::Properties};

                let vertex_size = std::mem::size_of::<DrawVert>();
                let index_size = std::mem::size_of::<DrawIdx>();

                let vertex = vertex::make_buffer::<B>(
                    device.as_ref(),
                    &adapter.physical_device,
                    vertex_size as usize * VTX_COUNT,
                    Usage::VERTEX,
                    Properties::CPU_VISIBLE | Properties::COHERENT,
                )
                .expect("failed to create ui vertex buffer");
                let index = vertex::make_buffer::<B>(
                    device.as_ref(),
                    &adapter.physical_device,
                    index_size as usize * IDX_COUNT,
                    Usage::INDEX,
                    Properties::CPU_VISIBLE | Properties::COHERENT,
                )
                .expect("failed to create ui index buffer");

                frames.push(FrameData {
                    vertex_buffer: vertex,
                    index_buffer: index,
                    pipeline: None,
                    matrix: glm::Mat4::identity(),
                    draw_infos: Vec::new(),
                    fb_width: 0.0,
                    fb_height: 0.0,
                    clip_off: [0.0, 0.0],
                    clip_scale: [0.0, 0.0],
                })
            }
        }

        Ok(ImGuiRenderer {
            platform,
            imgui: Some(imgui),
            last_frame: Instant::now(),
            window,
            device,
            frames,
            font_atlas,
            layout: description_set_layout,
            descriptor_pool: ManuallyDrop::new(descriptor_pool),
            descriptor_set,
        })
    }

    fn update_buffers(&self, draw_data: &DrawData, frame: &FrameData<B>) {
        if draw_data.total_vtx_count > VTX_COUNT as i32
            || draw_data.total_idx_count > IDX_COUNT as i32
        {
            error!(
                "Not enough vertex/index memory, is: vtx-{} idx-{}",
                draw_data.total_vtx_count, draw_data.total_idx_count
            );
        }

        unsafe {
            use gfx_hal::memory::Segment;

            let mapped_vertex_memory = self
                .device
                .map_memory(&frame.vertex_buffer.0, Segment::ALL)
                .expect("TODO");

            let mapped_index_memory = self
                .device
                .map_memory(&frame.index_buffer.0, Segment::ALL)
                .expect("TODO");

            let mut offset_bytes_vertex: usize = 0;
            let mut offset_bytes_index: usize = 0;

            for draw_list in draw_data.draw_lists() {
                let vertex_buffer = draw_list.vtx_buffer();
                let vertex_buffer_len = vertex_buffer.len() * std::mem::size_of::<DrawVert>();
                std::ptr::copy_nonoverlapping(
                    vertex_buffer.as_ptr() as *const u8,
                    mapped_vertex_memory.offset(offset_bytes_vertex as isize),
                    vertex_buffer_len,
                );

                let idx_buffer = draw_list.idx_buffer();
                let idx_buffer_len = idx_buffer.len() * std::mem::size_of::<DrawIdx>();
                std::ptr::copy_nonoverlapping(
                    idx_buffer.as_ptr() as *const u8,
                    mapped_index_memory.offset(offset_bytes_index as isize),
                    idx_buffer_len,
                );

                offset_bytes_vertex += vertex_buffer_len;
                offset_bytes_index += idx_buffer_len;
            }

            self.device
                .flush_mapped_memory_ranges(vec![
                    (&frame.vertex_buffer.0, Segment::ALL),
                    (&frame.index_buffer.0, Segment::ALL),
                ])
                .expect("TODO");

            self.device.unmap_memory(&frame.vertex_buffer.0);
            self.device.unmap_memory(&frame.index_buffer.0)
        }
    }

    pub fn new_frame(&mut self) -> UiHandle {
        UiHandle::new(self.imgui.take().expect("imgui was not initialized"))
    }

    pub fn update(&mut self, idx: usize, handle: UiHandle) {
        match handle {
            UiHandle { imgui, ui } => {
                self.platform.prepare_render(&ui, &self.window);
                let draw_data = ui.render();

                {
                    self.update_buffers(draw_data, &self.frames[idx])
                }

                {
                    let frame = self.frames.get_mut(idx).expect("frame not created");

                    let left = draw_data.display_pos[0];
                    let right = draw_data.display_pos[0] + draw_data.display_size[0];
                    let top = draw_data.display_pos[1];
                    let bottom = draw_data.display_pos[1] + draw_data.display_size[1];
                    frame.matrix = glm::ortho_rh_zo(left, right, top, bottom, -1.0, 1.0);

                    frame.fb_width = draw_data.display_size[0] * draw_data.framebuffer_scale[0];
                    frame.fb_height = draw_data.display_size[1] * draw_data.framebuffer_scale[1];

                    frame.clip_off = draw_data.display_pos;
                    frame.clip_scale = draw_data.framebuffer_scale;

                    frame.draw_infos.clear();

                    let mut vertex_offset = 0;
                    let mut index_offset: u32 = 0;
                    for draw_list in draw_data.draw_lists() {
                        for draw_cmd in draw_list.commands() {
                            match draw_cmd {
                                DrawCmd::Elements {
                                    count,
                                    cmd_params: DrawCmdParams { clip_rect, .. },
                                } => {
                                    frame.draw_infos.push(DrawInfo {
                                        clip_rect,
                                        count,
                                        vertex_offset,
                                        index_offset,
                                    });
                                    index_offset += count as u32;
                                }
                                _ => (),
                            }
                        }
                        vertex_offset += draw_list.vtx_buffer().len() as i32
                    }
                }
                self.imgui = Some(imgui);
            }
        }
    }

    pub fn render(&mut self, idx: usize, cmd: &mut B::CommandBuffer, shader: &ShaderSystem<B>) {
        {
            let pipeline = shader
                .get_pipeline("ui".to_string())
                .expect("failed to get ui pipeline");
            self.frames[idx].pipeline = Some(pipeline);
        }
        let frame = &self.frames[idx];
        unsafe {
            use gfx_hal::pso::ShaderStageFlags;

            cmd.bind_graphics_pipeline(&frame.pipeline.as_ref().expect("no pipeline").pipeline);

            cmd.bind_graphics_descriptor_sets(
                &frame
                    .pipeline
                    .as_ref()
                    .expect("no pipeline")
                    .pipeline_layout,
                0,
                Some(&self.descriptor_set),
                &[],
            );

            cmd.bind_vertex_buffers(
                0,
                vec![(
                    &frame.vertex_buffer.1 as &B::Buffer,
                    gfx_hal::buffer::SubRange::WHOLE,
                )],
            );
            cmd.bind_index_buffer(IndexBufferView {
                buffer: &frame.index_buffer.1,
                range: gfx_hal::buffer::SubRange::WHOLE,
                index_type: IndexType::U16,
            });
            cmd.push_graphics_constants(
                &frame
                    .pipeline
                    .as_ref()
                    .expect("no pipeline")
                    .pipeline_layout,
                ShaderStageFlags::VERTEX,
                0,
                push_constant_bytes(&frame.matrix),
            );
        }

        for draw_info in &frame.draw_infos {
            let clip_rect = [
                (draw_info.clip_rect[0] - frame.clip_off[0]) * frame.clip_scale[0],
                (draw_info.clip_rect[1] - frame.clip_off[1]) * frame.clip_scale[1],
                (draw_info.clip_rect[2] - frame.clip_off[0]) * frame.clip_scale[0],
                (draw_info.clip_rect[3] - frame.clip_off[1]) * frame.clip_scale[1],
            ];

            if clip_rect[0] < frame.fb_width
                && clip_rect[1] < frame.fb_height
                && clip_rect[2] >= 0.0
                && clip_rect[3] >= 0.0
            {
                let scissor = Rect {
                    x: f32::max(0.0, clip_rect[0]).floor() as i16,
                    y: f32::max(0.0, clip_rect[1]).floor() as i16,
                    w: (clip_rect[2] - clip_rect[0]).abs().ceil() as i16,
                    h: (clip_rect[3] - clip_rect[1]).abs().ceil() as i16,
                };
                unsafe {
                    cmd.set_scissors(0, &[scissor]);
                    cmd.draw_indexed(
                        draw_info.index_offset..draw_info.index_offset + draw_info.count as u32,
                        draw_info.vertex_offset,
                        0..1,
                    );
                }
            }
        }
    }

    // pub fn get_frame(&'lf self) -> Option<&Ui<'lf>> {
    //     // self.frame.as_ref()
    // }

    pub fn handle_event<T>(&mut self, event: &Event<T>) {
        match event {
            Event::NewEvents(_) => {
                self.last_frame = self
                    .imgui
                    .as_mut()
                    .expect("imgui not initialized")
                    .io_mut()
                    .update_delta_time(self.last_frame);
            }
            Event::MainEventsCleared => (),
            // self.platform
            //     .prepare_frame(self.imgui.as_mut().expect("").borrow_mut().io_mut(), self.window.borrow())
            //     .expect("failed to prepare frame");
            // let mut context = self.imgui.take().expect("");
            // self.frame = Some(OwningHandle::new_with_fn(context, |ctx| {
            // 		unsafe {
            // 				let cell: &RefCell<Context> = ctx.as_ref().expect("Pointer was null");
            // 		// let mut c: &mut Context = cell.borrow_mut();
            // 				Box::new(cell.borrow_mut().frame())
            // 		}
            // }));
            Event::RedrawRequested(_) => (),
            // self.platform.prepare_render(
            //     &self.frame.expect(""),
            //     self.window.borrow(),
            // );
            // let frame = self.frame.take().expect("");
            // let draw_data = frame.render();
            // self.imgui = Some(frame.into_owner());
            event => self.platform.handle_event(
                self.imgui.as_mut().expect("imgui not initialized").io_mut(),
                self.window.borrow(),
                &event,
            ),
        }
    }
}

impl<B: Backend> Drop for ImGuiRenderer<B> {
    fn drop(&mut self) {
        unsafe {
            for frame in self.frames.drain(..) {
                // destroy buffers
                self.device.destroy_buffer(frame.vertex_buffer.1);
                self.device.destroy_buffer(frame.index_buffer.1);

                // destroy memories
                self.device.free_memory(frame.vertex_buffer.0);
                self.device.free_memory(frame.index_buffer.0);
            }

            self.font_atlas.free(self.device.borrow());
            self.device
                .destroy_descriptor_pool(ManuallyDrop::take(&mut self.descriptor_pool));
        }
    }
}
