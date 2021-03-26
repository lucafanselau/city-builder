use app::{Resources, World};

use crate::prelude::GpuContext;

use self::builder::GraphBuilder;

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
