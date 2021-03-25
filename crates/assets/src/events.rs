use crate::prelude::{Asset, AssetHandle};

#[derive(Debug, Clone)]
pub enum AssetEvent<A: Asset> {
    Created(AssetHandle<A>),
    Updated(AssetHandle<A>),
    // TODO: Destroyed,
}

impl<A: Asset> AssetEvent<A> {
    pub fn get_handle(&self) -> &AssetHandle<A> {
        match self {
            AssetEvent::Created(handle) => handle,
            AssetEvent::Updated(handle) => handle,
        }
    }

    /// Returns `true` if the asset_event is [`Updated`].
    pub fn is_updated(&self) -> bool {
        matches!(self, Self::Updated(..))
    }

    /// Returns `true` if the asset_event is [`Created`].
    pub fn is_created(&self) -> bool {
        matches!(self, Self::Created(..))
    }
}
