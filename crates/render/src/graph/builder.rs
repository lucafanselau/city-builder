use generational_arena::Arena;

/**
 *  Utility function to make a graph runnable
 */
use crate::{
    graph::{node::Node, nodes::pass::PassNode, Graph},
    prelude::GpuContext,
    resource::render_pass::{Attachment, AttachmentLayout, SubpassDescriptor},
    util::format::TextureLayout,
};
use std::{ops::Range, sync::Arc};

use super::{attachment::GraphAttachment, nodes::pass::PassAttachment};

fn build_attachment(
    attachments: &Arena<GraphAttachment>,
    a: &PassAttachment,
    layout_cb: impl Fn(&GraphAttachment) -> Range<TextureLayout>,
) -> Attachment {
    let graph_attachment = attachments
        .get(a.index.clone())
        .expect("[PassNodeBuilder] failed to find output attachment");

    let layouts = layout_cb(graph_attachment);

    Attachment {
        format: graph_attachment.format.clone(),
        load_op: a.load.clone(),
        store_op: a.store.clone(),
        layouts,
    }
}

fn build_node<Context: GpuContext>(
    ctx: &Arc<Context>,
    attachments: &Arena<GraphAttachment>,
    n: Node<Context>,
) -> Node<Context> {
    match n {
        Node::PassNode(mut pass_node) => {
            // Basically we will just need to Render Pass for this
            let mut rp_attachments: Vec<Attachment> = pass_node
                .output_attachments
                .iter()
                .map(|a| {
                    build_attachment(attachments, a, |ga| {
                        let final_layout = if ga.is_backbuffer {
                            TextureLayout::Present
                        } else {
                            TextureLayout::ShaderReadOnlyOptimal
                        };
                        TextureLayout::Undefined..final_layout
                    })
                })
                .collect();

            rp_attachments.extend(pass_node.input_attachments.iter().map(|a| {
                build_attachment(attachments, a, |_a| {
                    TextureLayout::ShaderReadOnlyOptimal..TextureLayout::ShaderReadOnlyOptimal
                })
            }));

            if let Some(a) = &pass_node.depth_attachment {
                rp_attachments.push(build_attachment(attachments, a, |_| {
                    TextureLayout::DepthStencilAttachmentOptimal
                        ..TextureLayout::DepthStencilAttachmentOptimal
                }));
            }

            // Here we will only have one Subpass since atm
            // we map one PassNode to one Render Pass
            // We should think about extending this to a broader scale
            // which will yield performance benefits on mobile gpu's
            //
            // Attachments will be layed out in the following order
            // Output (eg. color): rp_attachments[0..num_of_out]
            // Input Attachments: rp_attachments[num_of_out..num_of_out + num_of_in]
            // Depth Attachment: rp_attachments[num_of_out + num_of_in]
            let num_of_out = pass_node.output_attachments.len();
            let num_of_in = pass_node.input_attachments.len();
            let subpass = SubpassDescriptor {
                colors: (0..num_of_out)
                    .into_iter()
                    .map(|i| (i, TextureLayout::ColorAttachmentOptimal))
                    .collect(),
                depth_stencil: pass_node.depth_attachment.map(|_| {
                    (
                        (num_of_out + num_of_in) as usize,
                        TextureLayout::DepthStencilAttachmentOptimal,
                    )
                }),
                inputs: (num_of_out..(num_of_out + num_of_in))
                    .into_iter()
                    .map(|i| (i, TextureLayout::ShaderReadOnlyOptimal))
                    .collect(),
                resolves: vec![],
                preserves: vec![],
            };

            unimplemented!()
        }
    }
}

fn build_graph<Context: GpuContext>(ctx: Arc<Context>, graph: &mut Graph<Context>) {
    let attachments = &graph.attachments;
    graph.nodes = graph
        .nodes
        .drain(..)
        .map(|n| build_node(&ctx, attachments, n))
        .collect();
}

pub fn maybe_build_graph<Context: GpuContext>(ctx: Arc<Context>, graph: &mut Graph<Context>) {
    if graph.is_dirty {
        build_graph(ctx, graph);
        graph.is_dirty = false;
    }
}
