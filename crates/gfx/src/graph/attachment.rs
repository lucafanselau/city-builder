use generational_arena::Index;
use gfx_hal::{
    device::Device,
    image::{Kind, Usage, ViewCapabilities},
    Backend,
};
use render::{
    graph::attachment::{AttachmentSize, GraphAttachment},
    resource::frame::Extent2D,
};

use crate::compat::ToHalType;

#[derive(Debug, Clone)]
pub enum AttachmentIndex {
    Backbuffer,
    Custom(Index),
}

pub(crate) struct GfxGraphAttachment<B: Backend> {
    desc: GraphAttachment,
    image: B::Image,
    image_view: B::ImageView,
}

impl<B: Backend> GfxGraphAttachment<B> {
    pub fn create(desc: GraphAttachment, device: &B::Device, dimension: Extent2D) -> Self {
        todo!()
    }

    /// Should be called whenever swapchain dimensions change
    pub fn rebuild(&mut self, device: &B::Device, dimension: Extent2D) {
        todo!()
    }

    fn build(
        desc: &GraphAttachment,
        device: &B::Device,
        dimension: Extent2D,
    ) -> (B::Image, B::ImageView) {
        let image = unsafe {
            // Calculate Size
            let (width, height) = match desc.size {
                AttachmentSize::Relative(scale_x, scale_y) => (
                    (dimension.width as f32 * scale_x) as u32,
                    (dimension.height as f32 * scale_y) as u32,
                ),
                AttachmentSize::Absolute(width, height) => (width, height),
            };
            let kind = Kind::D2(width, height, 0, 1);

            device
                .create_image(
                    kind,
                    1,
                    desc.format.convert(),
                    desc.tiling.convert(),
                    Usage::DEPTH_STENCIL_ATTACHMENT,
                    ViewCapabilities::empty(),
                )
                .expect("[GfxGraph] (build attachment) failed to create image")
        };

        todo!()
    }
}
