pub mod attachment;

// Disabled but should be reusable in gfx
// pub mod builder;
pub mod node;
pub mod nodes;
pub mod graph;

// pub struct Graph<Context: GpuContext> {
//     pub attachments: Arena<GraphAttachment>,
//     pub nodes: Vec<Node<Context>>,
//     pub is_dirty: bool,
// }

// impl<Context: GpuContext> Graph<Context> {
//     pub fn new() -> Self {
//         Self {
//             attachments: Arena::new(),
//             nodes: Vec::new(),
//             is_dirty: true,
//         }
//     }

//     pub fn add_attachment(
//         &mut self,
//         size: AttachmentSize,
//         format: TextureFormat,
//     ) -> AttachmentIndex {
//         self.is_dirty = true;
//         self.attachments.insert(GraphAttachment::new(size, format))
//     }

//     pub fn add_node(&mut self, n: Node<Context>) {
//         self.is_dirty = true;
//         self.nodes.push(n);
//     }
// }
