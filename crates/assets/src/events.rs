use crate::prelude::{Asset, AssetHandle};

/// An Asset Event, containing a weak handle to an asset
#[derive(Debug)]
pub enum AssetEvent<A: Asset> {
    Created(AssetHandle<A>),
    Updated(AssetHandle<A>),
    Destroyed(AssetHandle<A>),
}

impl<A: Asset> AssetEvent<A> {
    pub fn get_handle(&self) -> &AssetHandle<A> {
        match self {
            AssetEvent::Created(handle) => handle,
            AssetEvent::Updated(handle) => handle,
            AssetEvent::Destroyed(handle) => handle,
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

    /// Returns `true` if the asset_event is [`Destroyed`].
    pub fn is_destroyed(&self) -> bool {
        matches!(self, Self::Destroyed(..))
    }
}
