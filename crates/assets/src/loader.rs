use std::path::Path;

use crate::{asset_server::ChannelMap, handle::AssetHandleUntyped, BoxedFuture};

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
