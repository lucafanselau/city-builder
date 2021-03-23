use std::any::Any;

use async_channel::{unbounded, Receiver, Sender};

use crate::handle::{AssetHandle, AssetHandleUntyped};

pub trait Asset: Any + Send + Sync {}
impl<T: Any + Send + Sync> Asset for T {}
unsafe fn cast_asset<A: Asset>(asset: Box<dyn Asset>) -> Box<A> {
    // TODO: downcast_rs
    // # Safety:
    // Test if this is actually secure
    let asset: Box<dyn Any + 'static> = std::mem::transmute(asset);
    asset.downcast().expect("cast_asset error")
}

#[derive(Clone, Debug)]
pub struct AssetChannel {
    sender: Sender<(AssetHandleUntyped, Box<dyn Asset>)>,
    receiver: Receiver<(AssetHandleUntyped, Box<dyn Asset>)>,
}

impl AssetChannel {
    pub(crate) fn new() -> Self {
        let (sender, receiver) = unbounded();
        Self { sender, receiver }
    }

    pub(crate) async fn send<A: Asset + 'static>(&self, handle: AssetHandle<A>, a: A) {
        self.send_boxed(handle, Box::new(a)).await
    }

    pub(crate) async fn send_boxed<A: Asset + 'static>(&self, handle: AssetHandle<A>, a: Box<A>) {
        self.sender
            .send((handle.untyped(), a))
            .await
            .expect("AsyncChannel failed to send")
    }

    pub async unsafe fn receive<A: Asset>(&self) -> (AssetHandle<A>, Box<A>) {
        let (id, a) = self
            .receiver
            .recv()
            .await
            .expect("AssetChannel failed to receive");
        (id.typed(), cast_asset(a))
    }

    pub unsafe fn try_receive<A: Asset>(&self) -> Option<(AssetHandle<A>, Box<A>)> {
        self.receiver
            .try_recv()
            .ok()
            .map(|(id, a)| (id.typed(), cast_asset(a)))
    }
}
