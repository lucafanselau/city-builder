use std::{borrow::Cow, cell::RefCell};

use crate::{
    graph::builder::GraphBuilder,
    resource::render_pass::{LoadOp, StoreOp},
};

use super::callbacks::{InitCallback, PassCallback, PassCallbacks, PassCallbacksImpl, UserData};

#[derive(Clone)]
pub struct PassAttachment<I: Clone + Copy> {
    pub index: I,
    pub load: LoadOp,
    pub store: StoreOp,
}

pub struct PassNode<G: GraphBuilder + ?Sized> {
    pub name: Cow<'static, str>,
    pub output_attachments: Vec<PassAttachment<<G as GraphBuilder>::AttachmentIndex>>,
    pub input_attachments: Vec<PassAttachment<<G as GraphBuilder>::AttachmentIndex>>,
    pub depth_attachment: Option<PassAttachment<<G as GraphBuilder>::AttachmentIndex>>,
    pub callbacks: RefCell<Box<dyn PassCallbacks<<G as GraphBuilder>::Context>>>,
}

// impl<Context: 'static + GpuContext, U: Send + Sync + 'static> Node for PassNode<Context, U> {
//     fn node_type(&self) -> NodeType {
//         NodeType::Pass
//     }

//     fn inputs(&self) -> Vec<AttachmentIndex> {
//         let mut r: Vec<AttachmentIndex> = self.input_attachments.iter().map(|a| a.index.clone()).collect();
//         if let Some(d) = &self.depth_attachment {
//             r.push(d.index.clone())
//         }
//         r
//     }

//     fn outputs(&self) -> Vec<AttachmentIndex> {
//         self.output_attachments.iter().map(|a| a.index.clone()).collect()
//     }
// }

//
// Builder for Pass Nodes
//

pub struct PassNodeBuilder<G: GraphBuilder + ?Sized, U: UserData> {
    name: Cow<'static, str>,
    output_attachments: Vec<PassAttachment<<G as GraphBuilder>::AttachmentIndex>>,
    input_attachments: Vec<PassAttachment<<G as GraphBuilder>::AttachmentIndex>>,
    depth_attachment: Option<PassAttachment<<G as GraphBuilder>::AttachmentIndex>>,
    init: Option<Box<dyn InitCallback<<G as GraphBuilder>::Context, U>>>,
    cb: Option<Box<dyn PassCallback<<G as GraphBuilder>::Context, U>>>,
}

impl<G: GraphBuilder + ?Sized, U: UserData> PassNodeBuilder<G, U> {
    pub fn new(name: Cow<'static, str>) -> Self {
        Self {
            name,
            output_attachments: Vec::new(),
            input_attachments: Vec::new(),
            depth_attachment: None,
            init: None,
            cb: None,
        }
    }

    pub fn add_output(
        &mut self,
        index: <G as GraphBuilder>::AttachmentIndex,
        load: LoadOp,
        store: StoreOp,
    ) -> &mut Self {
        self.output_attachments
            .push(PassAttachment { index, load, store });
        self
    }

    pub fn add_input(
        &mut self,
        index: <G as GraphBuilder>::AttachmentIndex,
        load: LoadOp,
        store: StoreOp,
    ) -> &mut Self {
        self.input_attachments
            .push(PassAttachment { index, load, store });
        self
    }

    pub fn set_depth(
        &mut self,
        index: <G as GraphBuilder>::AttachmentIndex,
        load: LoadOp,
        store: StoreOp,
    ) -> &mut Self {
        self.depth_attachment = Some(PassAttachment { index, load, store });
        self
    }

    pub fn init(
        &mut self,
        func: Box<dyn InitCallback<<G as GraphBuilder>::Context, U> + 'static>,
    ) -> &mut Self {
        self.init = Some(func);
        self
    }

    pub fn callback(
        &mut self,
        func: Box<dyn PassCallback<<G as GraphBuilder>::Context, U> + 'static>,
    ) -> &mut Self {
        self.cb = Some(func);
        self
    }

    pub fn build(&mut self) -> PassNode<G>
    where
        <G as GraphBuilder>::Context: 'static,
    {
        let init = self
            .init
            .take()
            .expect("[PassNodeBuilder] (build) no init callback");

        let runner = self
            .cb
            .take()
            .expect("[PassNodeBuilder] (build) no init callback");

        let callbacks = PassCallbacksImpl::create(init, runner);

        PassNode {
            name: self.name.clone(),
            output_attachments: self.output_attachments.drain(..).collect(),
            input_attachments: self.input_attachments.drain(..).collect(),
            depth_attachment: self.depth_attachment.take(),
            callbacks: RefCell::new(Box::new(callbacks)),
        }
    }
}
