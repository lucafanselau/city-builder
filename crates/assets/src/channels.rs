use std::{any::TypeId, ops::Deref, sync::Arc};

use dashmap::{mapref::one::Ref, DashMap};
use tasks::channel::{Receiver, Sender};

use crate::{
    handle::{AssetHandleUntyped, HandleId},
    prelude::{Asset, AssetHandle},
};

type AssetPair = (HandleId, Box<dyn Asset>);

#[derive(Clone, Debug)]
pub struct AssetPipeSender(Sender<AssetPair>);

impl AssetPipeSender {
    pub async fn send(&self, event: AssetPair) -> Result<(), tasks::channel::SendError<AssetPair>> {
        self.0.send(event).await
    }
}

#[derive(Debug, Clone, Default)]
pub struct AssetSenderMap(Arc<DashMap<TypeId, AssetPipeSender>>);

impl AssetSenderMap {
    pub fn add_pipe<A: Asset>(&self, pipe: AssetPipeSender) {
        let type_id = std::any::TypeId::of::<A>();
        self.0.insert(type_id, pipe);
    }

    pub fn get_pipe<A: Asset>(&self) -> Ref<TypeId, AssetPipeSender> {
        let type_id = std::any::TypeId::of::<A>();
        self.0.get(&type_id).unwrap()
    }
}

#[derive(Debug, Clone)]
pub struct AssetPipeReceiver(Receiver<AssetPair>);

impl AssetPipeReceiver {
    pub async fn receive(&self) -> Result<AssetPair, tasks::channel::RecvError> {
        self.0.recv().await
    }

    pub fn try_receive(&self) -> Option<AssetPair> {
        self.0.try_recv().ok()
    }
}

#[derive(Debug, Clone, Default)]
pub struct AssetReceiverMap(Arc<DashMap<TypeId, AssetPipeReceiver>>);

impl AssetReceiverMap {
    pub fn add_pipe<A: Asset>(&self, pipe: AssetPipeReceiver) {
        let type_id = std::any::TypeId::of::<A>();
        self.0.insert(type_id, pipe);
    }

    pub fn get_pipe<A: Asset>(&self) -> Ref<TypeId, AssetPipeReceiver> {
        let type_id = std::any::TypeId::of::<A>();
        self.0.get(&type_id).unwrap()
    }
}

pub fn asset_pipe() -> (AssetPipeSender, AssetPipeReceiver) {
    let (sender, receiver) = tasks::channel::unbounded();
    (AssetPipeSender(sender), AssetPipeReceiver(receiver))
}

/// ****************************************
/// region: RefCounter Channels

#[derive(Debug, Clone)]
pub enum RefEvent {
    Increase { id: HandleId },
    Decrease { id: HandleId },
}

pub type RefReceiver = Receiver<RefEvent>;
pub type RefSender = Sender<RefEvent>;

pub fn ref_pipe() -> (RefSender, RefReceiver) {
    tasks::channel::unbounded()
}

#[derive(Debug, Clone, Default)]
pub struct RefCounterMap(Arc<DashMap<TypeId, (RefSender, RefReceiver)>>);

impl RefCounterMap {
    pub fn add_pipe<A: Asset>(&self) {
        let type_id = TypeId::of::<A>();
        self.0.insert(type_id, ref_pipe());
    }

    pub fn get_pipe<A: Asset>(&self) -> Option<impl Deref<Target = (RefSender, RefReceiver)> + '_> {
        let type_id = TypeId::of::<A>();
        self.get_pipe_from_type(type_id)
    }

    pub fn get_pipe_from_type(
        &self,
        type_id: TypeId,
    ) -> Option<impl Deref<Target = (RefSender, RefReceiver)> + '_> {
        self.0.get(&type_id)
    }
}
