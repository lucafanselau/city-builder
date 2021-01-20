use std::{borrow::Cow, marker::PhantomData, sync::Arc};

use app::{Resources, World};
use generational_arena::{Arena, Index};
use gfx_hal::Backend;
use render::graph::{attachment::GraphAttachment, graph::Graph, node::Node};

use crate::gfx_context::GfxContext;

pub mod attachment;
use self::{attachment::AttachmentIndex, nodes::GfxNode};

pub mod builder;
pub mod nodes;

pub struct GfxGraph<B: Backend> {
    ctx: Arc<GfxContext<B>>,
    attachments: Arena<GraphAttachment>,
    nodes: Arena<GfxNode<B>>,
}

impl<B: Backend> Graph for GfxGraph<B> {
    type Context = GfxContext<B>;

    type AttachmentIndex = AttachmentIndex;

    fn create(ctx: Arc<Self::Context>) -> Self {
        log::info!("CREATE");
        Self {
            ctx,
            attachments: Arena::new(),
            nodes: Arena::new(),
        }
    }

    fn add_node(&mut self, node: Node<Self>) {
        log::info!("ADD NODE");
        self.nodes.insert(builder::build_node(
            self.ctx.get_raw(),
            node,
            &self.attachments,
        ));
    }

    fn add_attachment(&mut self, attachment: GraphAttachment) -> Self::AttachmentIndex {
        log::info!("ADD ATTACHMENT");
        let index = self.attachments.insert(attachment);
        AttachmentIndex::Custom(index)
    }

    fn attachment_index(&self, name: Cow<'static, str>) -> Option<Self::AttachmentIndex> {
        self.attachments
            .iter()
            .find(|(_i, a)| a.name == name)
            .map(|(i, _a)| AttachmentIndex::Custom(i))
    }

    fn get_backbuffer_attachment(&self) -> Self::AttachmentIndex {
        log::info!("GET BACKBUFFER ATTACHMENT");
        AttachmentIndex::Backbuffer
    }

    fn execute(&mut self, world: &mut World, resources: &mut Resources) {
        log::info!("EXECUTE");
        todo!()
    }
}

// TODO: Drop the custom nodes
