//! Should contain all relevant structs for internal render pass creation
//! This is really shitty atm, since we will basically just support single pass render_passes
//! Maybe we should just settle for a system like a render_pass

use crate::resource::pipeline::PipelineStage;
use crate::util::format::{ImageAccess, TextureFormat, TextureLayout};
use std::ops::Range;

pub type AttachmentLayout = TextureLayout;

#[derive(Debug, Clone)]
pub enum AttachmentLoadOp {
    Load,
    Clear,
    DontCare,
}

#[derive(Debug, Clone)]
pub enum AttachmentStoreOp {
    Store,
    DontCare,
}

#[derive(Debug, Clone)]
pub struct Attachment {
    pub format: TextureFormat,
    // TODO: Multisampling
    pub load_op: AttachmentLoadOp,
    pub store_op: AttachmentStoreOp,
    pub layouts: Range<AttachmentLayout>,
}

pub type AttachmentRef = (usize, AttachmentLayout);

#[derive(Debug, Clone, Default)]
pub struct SubpassDescriptor {
    pub colors: Vec<AttachmentRef>,
    pub depth_stencil: Option<AttachmentRef>,
    pub inputs: Vec<AttachmentRef>,
    pub resolves: Vec<AttachmentRef>,
    pub preserves: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct SubpassDependency {
    /// Meaning this is a dependency fromSrc..toDst, where None is equal to VK_SUBPASS_EXTERNAL
    pub passes: Range<Option<u8>>,
    pub stages: Range<PipelineStage>,
    pub accesses: Range<ImageAccess>,
}

/// Describes (essentially a gfx-hal) render pass, should be primarily used by render graph implementation
///
/// Please not that on this specific part of the Render API next to none abstraction is provided,
/// since after implementing the Render Graph system, we should never think about constructing render
/// passes again
#[derive(Debug, Clone)]
pub struct RenderPassDescriptor {
    pub attachments: Vec<Attachment>,
    pub subpasses: Vec<SubpassDescriptor>,
    pub pass_dependencies: Vec<SubpassDependency>,
}

#[cfg(test)]
mod tests {

    #[test]
    fn build_render_pass() {}
}
