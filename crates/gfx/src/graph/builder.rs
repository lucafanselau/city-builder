use std::ops::Range;

use generational_arena::{Arena, Index};
use gfx_hal::{
    device::Device,
    image::Layout,
    pass::{Attachment, AttachmentOps, AttachmentRef, SubpassDependency, SubpassDesc},
    Backend,
};
use render::{
    graph::{attachment::GraphAttachment, node::Node, nodes::pass::PassNode},
    resource::render_pass::{LoadOp, StoreOp},
};

use crate::compat::ToHalType;

use super::{
    attachment::AttachmentIndex,
    nodes::{GfxNode, GfxPassNode},
    GfxGraph,
};

pub(super) fn build_node<B: Backend>(
    ctx: &B::Device,
    node: Node<GfxGraph<B>>,
    attachments: &Arena<GraphAttachment>,
) -> GfxNode<B> {
    match node {
        Node::PassNode(n) => GfxNode::PassNode(build_pass_node(ctx, n, attachments)),
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
        .get(index.clone())
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
    node: PassNode<GfxGraph<B>>,
    graph_attachments: &Arena<GraphAttachment>,
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
        AttachmentIndex::Backbuffer => {
            todo!()
        }
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

    GfxPassNode {
        graph_node: node,
        render_pass,
    }
}
