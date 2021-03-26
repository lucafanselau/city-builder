use std::borrow::Borrow;

use gfx_hal::{
    device::Device,
    format::{Aspects, Swizzle},
    image::{Kind, SubresourceRange, Usage, ViewCapabilities, ViewKind},
    Backend,
};
use render::{
    graph::{
        attachment::{AttachmentSize, GraphAttachment},
        nodes::pass::{PassAttachment, PassNode},
    },
    prelude::MemoryType,
    resource::frame::Extent2D,
};
use uuid::Uuid;

use crate::{
    compat::ToHalType,
    heapy::{AllocationIndex, Heapy},
};

use super::builder::GfxGraphBuilder;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AttachmentIndex {
    Backbuffer,
    Custom(Uuid),
}

pub(crate) struct GfxGraphAttachment<B: Backend> {
    pub(crate) desc: GraphAttachment,
    pub(crate) image: (B::Image, AllocationIndex),
    pub(crate) image_view: B::ImageView,
}

pub trait NodeIterator<B: Backend> = Iterator<Item: Borrow<PassNode<GfxGraphBuilder<B>>>>;

impl<B: Backend> GfxGraphAttachment<B> {
    pub fn create<I: NodeIterator<B>>(
        desc: GraphAttachment,
        device: &B::Device,
        heapy: &Heapy<B>,
        dimension: Extent2D,
        nodes: I,
    ) -> Self {
        let (image, image_view) = Self::build(&desc, device, heapy, dimension, nodes);
        Self {
            desc,
            image,
            image_view,
        }
    }

    /// Should be called whenever swapchain dimensions change
    pub fn rebuild<I: NodeIterator<B>>(
        &mut self,
        device: &B::Device,
        heapy: &Heapy<B>,
        dimension: Extent2D,
        nodes: I,
    ) {
        let (image, image_view) = Self::build(&self.desc, device, heapy, dimension, nodes);
        self.image = image;
        self.image_view = image_view;
    }

    fn build<I: NodeIterator<B>>(
        desc: &GraphAttachment,
        device: &B::Device,
        heapy: &Heapy<B>,
        dimension: Extent2D,
        nodes: I,
    ) -> ((B::Image, AllocationIndex), B::ImageView) {
        // Figure out usage
        let usage = {
            let mut res = Usage::empty();
            nodes.for_each(|n| {
                let node = n.borrow();

                let mut check_array = |a: &Vec<PassAttachment<AttachmentIndex>>, usage: Usage| {
                    if a.iter()
                        .map(|a| a.index)
                        .any(|i| i == AttachmentIndex::Custom(desc.id))
                    {
                        res |= usage
                    }
                };

                check_array(&node.output_attachments, Usage::COLOR_ATTACHMENT);
                check_array(&node.input_attachments, Usage::INPUT_ATTACHMENT);

                if node
                    .depth_attachment
                    .clone()
                    .filter(|p| p.index == AttachmentIndex::Custom(desc.id))
                    .is_some()
                {
                    res |= Usage::DEPTH_STENCIL_ATTACHMENT;
                }
            });
            res
        };

        log::info!(
            "For attachment [{}] we calculated a usage of: {:?}",
            desc.name,
            usage
        );

        let image = unsafe {
            // Calculate Size
            let (width, height) = match desc.size {
                AttachmentSize::Relative(scale_x, scale_y) => (
                    (dimension.width as f32 * scale_x) as u32,
                    (dimension.height as f32 * scale_y) as u32,
                ),
                AttachmentSize::Absolute(width, height) => (width, height),
            };
            let kind = Kind::D2(width, height, 1, 1);

            let mut image = device
                .create_image(
                    kind,
                    1,
                    desc.format.convert(),
                    desc.tiling.convert(),
                    usage,
                    ViewCapabilities::empty(),
                )
                .expect("[GfxGraph] (build attachment) failed to create image");

            // Bind image to some data
            let requirements = device.get_image_requirements(&image);
            let image_allocation = heapy.alloc(
                requirements.size,
                MemoryType::DeviceLocal,
                Some(requirements),
            );

            heapy.bind_image(&image_allocation, &mut image);

            (image, image_allocation)
        };

        // TODO: bind image memory

        let image_view = unsafe {
            let kind = ViewKind::D2;
            let swizzle = Swizzle::NO;
            let aspects = if usage.contains(Usage::DEPTH_STENCIL_ATTACHMENT) {
                if desc.format.has_stencil() {
                    Aspects::STENCIL | Aspects::DEPTH
                } else {
                    Aspects::DEPTH
                }
            } else {
                Aspects::COLOR
            };
            let range = SubresourceRange {
                aspects,
                level_start: 0,
                level_count: None,
                layer_start: 0,
                layer_count: None,
            };
            device
                .create_image_view(&image.0, kind, desc.format.convert(), swizzle, range)
                .expect("[GfxGraph] (build attachment) failed to create image view")
        };

        (image, image_view)
    }
}
