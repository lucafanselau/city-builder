use std::any::Any;

use async_channel::{unbounded, Receiver, Sender};
use downcast_rs::{impl_downcast, DowncastSync};

use crate::handle::{AssetHandle, AssetHandleUntyped};

pub trait Asset: DowncastSync {}
impl<T: DowncastSync> Asset for T {}
impl_downcast!(sync Asset);

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

    pub async unsafe fn receive<A: Asset>(&self) -> Option<(AssetHandle<A>, Box<A>)> {
        self.receiver
            .recv()
            .await
            .ok()
            .map(|(h, a)| a.downcast().ok().map(|v| (h.typed(), v)))
            .flatten()
    }

    pub unsafe fn try_receive<A: Asset>(&self) -> Option<(AssetHandle<A>, Box<A>)> {
        self.receiver
            .try_recv()
            .ok()
            .map(|(h, a)| a.downcast().ok().map(|v| (h.typed(), v)))
            .flatten()
    }
}
