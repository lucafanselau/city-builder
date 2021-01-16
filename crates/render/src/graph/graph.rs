use std::{borrow::Cow, sync::Arc};

use app::{Resources, World};

use crate::prelude::GpuContext;

use super::{attachment::GraphAttachment, node::Node, nodes::{callbacks::UserData, pass::{PassNodeBuilder}}};

pub trait Graph {

    type Context: GpuContext;
    // TODO: Trait bound is not very usefull here
    type AttachmentIndex: Sized;

    fn create(ctx: Arc<Self::Context>) -> Self;

    fn add_node(&mut self, node: Node<Self>);
    fn add_attachment(&mut self, attachment: GraphAttachment);

    fn get_backbuffer_attachment(&self) -> Self::AttachmentIndex;

    fn execute(&mut self, world: &mut World, resources: &mut Resources);

    fn build_pass_node<U: UserData>(&self, name: Cow<'static, str>) -> PassNodeBuilder<Self, U> {
        PassNodeBuilder::new(name)
    }

}
