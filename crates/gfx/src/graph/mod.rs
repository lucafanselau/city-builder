use std::marker::PhantomData;

use app::{Resources, World};
use generational_arena::{Arena, Index};
use gfx_hal::Backend;
use render::graph::graph::Graph;

use crate::gfx_context::GfxContext;



pub struct GfxGraph<B: Backend> {
    attachments: Arena<()>,
    _marker: PhantomData<B>
}

impl<B: Backend> Graph for GfxGraph<B> {
    type Context = GfxContext<B>;

    type AttachmentIndex = Index;

    fn create(ctx: std::sync::Arc<Self::Context>) -> Self {
        todo!()
    }

    fn add_node(&mut self, node: render::graph::node::Node<Self>) {
        todo!()
    }

    fn add_attachment(&mut self, attachment: render::graph::attachment::GraphAttachment) {
        todo!()
    }

    fn get_backbuffer_attachment(&self) -> Self::AttachmentIndex {
        todo!()
    }

    fn execute(&mut self, world: &mut World, resources: &mut Resources) {
        todo!()
    }
}
