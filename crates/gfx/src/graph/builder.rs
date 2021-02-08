use std::{
    convert::TryInto,
    mem::ManuallyDrop,
    ops::Deref,
    sync::{atomic::AtomicBool, Arc},
};

use generational_arena::{Arena, Index};
use gfx_hal::{
    adapter::Adapter,
    device::Device,
    format::{ChannelType, Format, ImageFeature},
    pool::CommandPoolCreateFlags,
    prelude::PhysicalDevice,
    window::Surface,
    Backend,
};
use parking_lot::{Mutex, RwLock};
use render::{
    graph::{
        attachment::GraphAttachment,
        builder::GraphBuilder,
        node::{self, Node},
    },
    resource::frame::Extent2D,
    util::format::TextureFormat,
};

use crate::{
    compat::ToHalType,
    context::{GfxContext, Queues},
};

use super::{
    attachment::AttachmentIndex, nodes::GfxNode, FrameStatus, FrameSynchronization, GfxGraph,
    GraphData,
};

pub struct GfxGraphBuilder<B: Backend> {
    data: GraphData<B>,
    attachments: Arena<GraphAttachment>,
    nodes: Arena<Node<Self>>,
}

impl<B: Backend> GfxGraphBuilder<B> {
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

        let depth_format = vec![
            TextureFormat::Depth32Sfloat,
            TextureFormat::Depth24PlusStencil8,
        ]
        .into_iter()
        .find(|f| -> bool {
            let properties = adapter.physical_device.format_properties(Some(f.convert()));
            properties
                .optimal_tiling
                .contains(ImageFeature::DEPTH_STENCIL_ATTACHMENT)
        })
        .expect("[GfxGraph] failed to find depth format");

        let graphics_family = queues.graphics_family;
        let command_pool = unsafe {
            device
                .create_command_pool(graphics_family, CommandPoolCreateFlags::RESET_INDIVIDUAL)
                .expect("[Swapper] failed to create command_pool")
        };

        let data = GraphData {
            device,
            surface,
            adapter,
            queues,
            depth_format,
            surface_format,
            surface_extent: RwLock::new(extent),
            frames_in_flight: 3u32,
            command_pool: ManuallyDrop::new(Mutex::new(command_pool)),
        };

        Self {
            data,
            attachments: Default::default(),
            nodes: Default::default(),
        }
    }
}

impl<B: Backend> GraphBuilder for GfxGraphBuilder<B> {
    type Context = GfxContext<B>;
    type AttachmentIndex = AttachmentIndex;
    type Graph = GfxGraph<B>;

    fn add_node(&mut self, node: Node<Self>) {
        self.nodes.insert(node);
    }

    fn add_attachment(&mut self, attachment: GraphAttachment) -> Self::AttachmentIndex {
        AttachmentIndex::Custom(self.attachments.insert(attachment))
    }

    fn attachment_index(
        &self,
        name: std::borrow::Cow<'static, str>,
    ) -> Option<Self::AttachmentIndex> {
        self.attachments
            .iter()
            .find(|(i, a)| a.name == name)
            .map(|(i, a)| AttachmentIndex::Custom(i))
    }

    fn get_backbuffer_attachment(&self) -> Self::AttachmentIndex {
        AttachmentIndex::Backbuffer
    }

    fn get_surface_format(&self) -> TextureFormat {
        self.data.surface_format
    }

    fn default_depth_format(&self) -> TextureFormat {
        self.data.depth_format
    }

    fn get_swapchain_image_count(&self) -> usize {
        self.data.frames_in_flight as _
    }

    fn build(self) -> Self::Graph {
        let GfxGraphBuilder {
            attachments,
            nodes,
            data,
        } = self;

        let frames_in_flight = data.frames_in_flight;
        let mut frames = Vec::with_capacity(frames_in_flight.try_into().unwrap());
        unsafe {
            for _ in 0..frames_in_flight {
                frames.push(ManuallyDrop::new(Mutex::new(
                    FrameSynchronization::<B>::create(&data.device),
                )));
            }
        }

        // TODO: Build attachments
        // Build nodes
        let nodes: Arena<GfxNode<B>> = nodes
            .into_iter()
            .map(|n| {
                super::nodes::build_node(data.device.deref(), n, &attachments, data.surface_format)
            })
            .collect();

        GfxGraph {
            attachments,
            nodes,
            data,
            should_configure_swapchain: AtomicBool::new(true),
            current_frame: RwLock::new((0, FrameStatus::Inactive)),
            frames,
        }
    }
}
