use std::{borrow::Cow, path::Path};

use crate::{
    channels::{AssetSenderMap, RefCounterMap},
    handle::{AssetHandleUntyped, HandleId},
    path::AssetPath,
    prelude::{Asset, AssetHandle},
    BoxedFuture,
};

/// Context of a load operations
pub struct LoadContext<'a> {
    pub senders: AssetSenderMap,
    pub ref_map: RefCounterMap,
    pub path: &'a Path,
    pub handle: HandleId,
}

impl<'a> LoadContext<'a> {
    pub fn new(
        senders: AssetSenderMap,
        ref_map: RefCounterMap,
        path: &'a Path,
        handle: HandleId,
    ) -> Self {
        Self {
            senders,
            ref_map,
            path,
            handle,
        }
    }

    pub async fn send_asset<A: Asset>(&self, asset: A) {
        self.senders
            .get_pipe::<A>()
            .value()
            .send((self.handle, Box::new(asset)))
            .await;
    }

    pub async fn add_asset_with_label<A: Asset>(
        &self,
        label: impl Into<&str>,
        asset: A,
    ) -> AssetHandle<A> {
        let asset_path =
            AssetPath::new(Cow::Borrowed(self.path), Some(Cow::Borrowed(label.into())));

        let id = HandleId::from_asset_path(asset_path);

        self.senders
            .get_pipe::<A>()
            .value()
            .send((id, Box::new(asset)))
            .await;

        let ref_pipe = self
            .ref_map
            .get_pipe::<A>()
            .expect("[LoadContext] (add_asset_with_label) failed to get ref_sender");

        AssetHandle::strong(id, ref_pipe.0.clone())
    }
}

/// Trait that needs to be implemented for every specific assets
pub trait AssetLoader: Send + Sync {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        ctx: LoadContext<'a>,
    ) -> BoxedFuture<'a, core::anyhow::Result<()>>;
    //maybe we'll need something like
    // (async) fn free(&self, ...)
    fn ext(&self) -> &[&str];
}
