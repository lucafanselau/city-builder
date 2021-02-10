use std::borrow::Cow;

use app::{Resources, World};

use crate::{prelude::GpuContext, util::format::TextureFormat};

use self::{
    attachment::GraphAttachment,
    builder::GraphBuilder,
    node::Node,
    nodes::{callbacks::UserData, pass::PassNodeBuilder},
};

pub mod attachment;
pub mod builder;
pub mod node;
pub mod nodes;

// TODO: Send + Sync (mainly pass callbacks are destroying that currently)
pub trait Graph {
    type Context: GpuContext;
    type AttachmentIndex: Clone + Copy;
    type Builder: GraphBuilder;

    fn execute(&mut self, world: &World, resources: &Resources);

    /// This function will remove all the prebuild things and strips down to the builder graph
    fn into_builder(self) -> Self::Builder;
}
