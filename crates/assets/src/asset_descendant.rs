use std::ops::Deref;

use ecs::prelude::{Events, Resources};

use crate::prelude::{Asset, AssetEvent, AssetHandle, Assets};

// TODO: Async ?
pub struct AssetDescendant<A: Asset, T> {
    cb: Box<dyn Fn(Vec<&A>) -> T + Send + Sync>,
    handles: Vec<AssetHandle<A>>,
    value: Option<T>,
}

impl<A: Asset, T> AssetDescendant<A, T> {
    pub fn new(
        cb: impl Fn(Vec<&A>) -> T + Send + Sync + 'static,
        handles: &[AssetHandle<A>],
    ) -> Self {
        Self {
            cb: Box::new(cb),
            handles: handles.to_vec(),
            value: None,
        }
    }

    pub fn get(&mut self, resources: &Resources) -> Option<&T> {
        // If an asset changes, we invalidate the cache
        let events = resources
            .get::<Events<AssetEvent<A>>>()
            .expect("[AssetDescendant] failed to get asset events");

        if events
            .iter()
            .filter(|a| a.is_updated())
            .map(|e| e.get_handle())
            .any(|h| self.handles.contains(h))
        {
            self.value = None
        }

        if self.value.is_none() {
            // check if all assets are present
            let assets = resources
                .get::<Assets<A>>()
                .expect("[AssetDescendant] failed to get assets store");

            let assets = assets.query(self.handles.as_slice());
            if let Ok(assets) = assets {
                let result = (self.cb)(assets.iter().map(|a| a.deref()).collect());
                self.value = Some(result);
            }
        }

        self.value.as_ref()
    }
}
