use gfx_hal::Backend;
use render::graph::nodes::pass::PassNode;

use super::GfxGraph;

//
// Custom Gfx Related Graph Nodes
// Basically a container around the render graph nodes
// With the gfx types to make them work
//

pub(crate) enum GfxNode<B: Backend> {
    PassNode(GfxPassNode<B>),
}

pub struct GfxPassNode<B: Backend> {
    pub(crate) graph_node: PassNode<GfxGraph<B>>,
    pub(crate) render_pass: B::RenderPass,
}
