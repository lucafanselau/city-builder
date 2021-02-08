use super::{builder::GraphBuilder, nodes::pass::PassNode};

pub enum Node<G>
where
    G: GraphBuilder + ?Sized,
{
    PassNode(PassNode<G>),
}

// pub trait Node: Downcast {
//     fn node_type(&self) -> NodeType;
//     fn inputs(&self) -> Vec<AttachmentIndex>;
//     fn outputs(&self) -> Vec<AttachmentIndex>;
// }
// impl_downcast!(Node);
