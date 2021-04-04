use std::{borrow::Cow, path::Path};

use crate::{
    asset_server::ChannelMap,
    handle::{AssetHandleUntyped, HandleId},
    path::AssetPath,
    prelude::{Asset, AssetHandle},
    BoxedFuture,
};

/// Context of a load operations
pub struct LoadContext<'a> {
    pub channels: ChannelMap,
    pub path: &'a Path,
    pub handle: AssetHandleUntyped,
}

impl<'a> LoadContext<'a> {
    pub fn new(channels: ChannelMap, path: &'a Path, handle: AssetHandleUntyped) -> Self {
        Self {
            channels,
            path,
            handle,
        }
    }

    pub async fn send_asset<A: Asset>(&self, asset: A) {
        self.channels
            .get_channel::<A>()
            .value()
            .send_untyped(self.handle.clone(), Box::new(asset))
            .await;
    }

    pub async fn add_asset_with_label<A: Asset>(
        &self,
        label: impl Into<&str>,
        asset: A,
    ) -> AssetHandle<A> {
        let asset_path =
            AssetPath::new(Cow::Borrowed(self.path), Some(Cow::Borrowed(label.into())));
        let handle = AssetHandleUntyped::new(HandleId::from_asset_path(asset_path));

        self.channels
            .get_channel::<A>()
            .value()
            .send_untyped(handle.clone(), Box::new(asset))
            .await;

        handle.typed()
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
