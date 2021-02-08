use gfx_hal::Backend;
use render::graph::nodes::pass::PassNode;

use std::ops::Range;

use generational_arena::{Arena, Index};
use gfx_hal::{
    device::Device,
    image::Layout,
    pass::{Attachment, AttachmentOps, AttachmentRef, SubpassDependency, SubpassDesc},
};
use render::{
    graph::{attachment::GraphAttachment, node::Node},
    resource::render_pass::{LoadOp, StoreOp},
    util::format::TextureFormat,
};

use crate::compat::ToHalType;

use super::{attachment::AttachmentIndex, builder::GfxGraphBuilder, GfxGraph};

//
// Custom Gfx Related Graph Nodes
// Basically a container around the render graph nodes
// With the gfx types to make them work
//

pub(crate) enum GfxNode<B: Backend> {
    PassNode(GfxPassNode<B>),
}

pub struct GfxPassNode<B: Backend> {
    pub(crate) graph_node: PassNode<GfxGraphBuilder<B>>,
    pub(crate) render_pass: B::RenderPass,
}

pub(super) fn build_node<B: Backend>(
    ctx: &B::Device,
    node: Node<GfxGraphBuilder<B>>,
    attachments: &Arena<GraphAttachment>,
    surface_format: TextureFormat,
) -> GfxNode<B> {
    match node {
        Node::PassNode(n) => {
            GfxNode::PassNode(build_pass_node(ctx, n, attachments, surface_format))
        }
    }
}

fn build_attachment<B: Backend>(
    attachments: &Arena<GraphAttachment>,
    index: Index,
    load: LoadOp,
    store: StoreOp,
    layouts: Range<Layout>,
) -> Attachment {
    let graph_attachment = attachments
        .get(index)
        .expect("[PassNodeBuilder] failed to find output attachment");

    Attachment {
        format: Some(graph_attachment.format.clone().convert()),
        // TODO: Multisampling
        samples: 1u8,
        ops: AttachmentOps::new(load.convert(), store.convert()),
        stencil_ops: AttachmentOps::DONT_CARE,
        layouts,
    }
}

fn build_pass_node<B: Backend>(
    ctx: &B::Device,
    mut node: PassNode<GfxGraphBuilder<B>>,
    graph_attachments: &Arena<GraphAttachment>,
    surface_format: TextureFormat,
) -> GfxPassNode<B> {
    // ctx.create_render_pass(attachments, subpasses, dependencies);

    let num_of_out = node.output_attachments.len();
    let num_of_in = node.input_attachments.len();
    let has_depth = node.depth_attachment.is_some();

    let mut attachments: Vec<Attachment> =
        Vec::with_capacity(num_of_out + num_of_in + (if has_depth { 1 } else { 0 }));

    attachments.extend(node.output_attachments.iter().map(|a| match a.index {
        AttachmentIndex::Custom(index) => build_attachment::<B>(
            graph_attachments,
            index,
            a.load.clone(),
            a.store.clone(),
            Layout::Undefined..Layout::ShaderReadOnlyOptimal,
        ),
        AttachmentIndex::Backbuffer => Attachment {
            format: Some(surface_format.clone().convert()),
            samples: 1u8,
            ops: AttachmentOps::new(a.load.clone().convert(), a.store.clone().convert()),
            stencil_ops: AttachmentOps::DONT_CARE,
            layouts: Layout::Undefined..Layout::Present,
        },
    }));

    attachments.extend(node.input_attachments.iter().map(|a| match a.index {
        AttachmentIndex::Custom(index) => build_attachment::<B>(
            graph_attachments,
            index,
            a.load.clone(),
            a.store.clone(),
            Layout::ShaderReadOnlyOptimal..Layout::ShaderReadOnlyOptimal,
        ),
        AttachmentIndex::Backbuffer => {
            panic!("Backbuffer as input attachment in graph is not allowed")
        }
    }));

    if let Some(a) = &node.depth_attachment {
        attachments.push(match a.index {
            AttachmentIndex::Custom(index) => build_attachment::<B>(
                graph_attachments,
                index,
                a.load.clone(),
                a.store.clone(),
                Layout::DepthStencilAttachmentOptimal..Layout::DepthStencilAttachmentOptimal,
            ),
            AttachmentIndex::Backbuffer => {
                panic!("Backbuffer as depth attachment in graph is not allowed")
            }
        })
    }

    let create_attachment_ref = |range: Range<usize>, layout: Layout| -> Vec<AttachmentRef> {
        range.into_iter().map(|i| (i, layout)).collect()
    };

    let depth_stencil = if node.depth_attachment.is_some() {
        Some((
            num_of_out + num_of_in,
            Layout::DepthStencilAttachmentOptimal,
        ))
    } else {
        None
    };

    let subpass = SubpassDesc {
        colors: &create_attachment_ref(0..num_of_out, Layout::ColorAttachmentOptimal),
        depth_stencil: depth_stencil.as_ref(),
        inputs: &create_attachment_ref(
            num_of_out..(num_of_out + num_of_in),
            Layout::ShaderReadOnlyOptimal,
        ),
        resolves: &Vec::new(),
        preserves: &Vec::new(),
    };

    let dependencies: Vec<SubpassDependency> = Vec::new();

    let render_pass = unsafe {
        ctx.create_render_pass(attachments, &vec![subpass], dependencies)
            .expect("Failed to build PassNode")
    };

    node.callbacks.init(&render_pass);

    GfxPassNode {
        graph_node: node,
        render_pass,
    }
}
