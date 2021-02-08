use std::borrow::Cow;

use crate::{prelude::GpuContext, util::format::TextureFormat};

use super::{
    attachment::GraphAttachment,
    node::Node,
    nodes::{callbacks::UserData, pass::PassNodeBuilder},
    Graph,
};

pub trait GraphBuilder {
    type Context: GpuContext;
    type AttachmentIndex: Clone;
    type Graph: Graph;

    fn add_node(&mut self, node: Node<Self>);
    fn add_attachment(&mut self, attachment: GraphAttachment) -> Self::AttachmentIndex;
    fn attachment_index(&self, name: Cow<'static, str>) -> Option<Self::AttachmentIndex>;

    fn get_backbuffer_attachment(&self) -> Self::AttachmentIndex;

    fn get_surface_format(&self) -> TextureFormat;
    fn default_depth_format(&self) -> TextureFormat;
    fn get_swapchain_image_count(&self) -> usize;

    fn build_pass_node<U: UserData>(&self, name: Cow<'static, str>) -> PassNodeBuilder<Self, U> {
        PassNodeBuilder::new(name)
    }

    fn build(self) -> Self::Graph;
}
