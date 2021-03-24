use crate::prelude::{Asset, AssetHandle};

#[derive(Debug, Clone)]
pub enum AssetEvent<A: Asset> {
    Created(AssetHandle<A>),
    Updated(AssetHandle<A>),
    // TODO: Destroyed,
}
