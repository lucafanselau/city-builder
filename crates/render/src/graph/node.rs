use super::{nodes::pass::PassNode, Graph};

pub enum Node<G>
where
    G: Graph + ?Sized,
{
    PassNode(PassNode<G>),
}

// pub trait Node: Downcast {
//     fn node_type(&self) -> NodeType;
//     fn inputs(&self) -> Vec<AttachmentIndex>;
//     fn outputs(&self) -> Vec<AttachmentIndex>;
// }
// impl_downcast!(Node);
