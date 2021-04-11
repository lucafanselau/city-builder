use std::{any::type_name, hash::BuildHasherDefault};

use crate::{
    asset::Asset,
    channels::{AssetPipeReceiver, RefEvent, RefReceiver},
    handle::{AssetHandle, HandleId},
    prelude::AssetEvent,
};
use core::anyhow::anyhow;
use dashmap::{mapref::one::Ref, DashMap};
use ecs::prelude::{Events, Res, ResMut};
use hash_hasher::{HashBuildHasher, HashHasher};

type Hasher = BuildHasherDefault<HashHasher>;

/// Simple Collection for Assets
pub struct Assets<A: Asset> {
    store: DashMap<HandleId, A, Hasher>,
    ref_count: DashMap<HandleId, u32, Hasher>,
    receiver: AssetPipeReceiver,
    ref_receiver: RefReceiver,
}

impl<A: Asset> Assets<A> {
    pub(crate) fn new(receiver: AssetPipeReceiver, ref_receiver: RefReceiver) -> Self {
        Self {
            store: DashMap::with_hasher(HashBuildHasher::default()),
            ref_count: DashMap::with_hasher(HashBuildHasher::default()),
            receiver,
            ref_receiver,
        }
    }

    pub fn query<'a>(
        &'a self,
        handles: &[AssetHandle<A>],
    ) -> core::anyhow::Result<Vec<Ref<'a, HandleId, A, Hasher>>> {
        let assets: Vec<Ref<HandleId, A, BuildHasherDefault<HashHasher>>> = handles
            .iter()
            .filter_map(|h| self.store.get(&h.id))
            .collect();
        if assets.len() == handles.len() {
            Ok(assets)
        } else {
            Err(anyhow!("failed to query all handles"))
        }
    }

    pub fn try_get(&self, handle: &AssetHandle<A>) -> Option<Ref<HandleId, A, Hasher>> {
        self.store.get(&handle.id)
    }

    pub fn get(&self, handle: &AssetHandle<A>) -> Ref<HandleId, A, Hasher> {
        self.try_get(handle)
            .expect("[Assets] failed to retrieve asset, maybe you want to use try_get?")
    }

    // TODO: Asset Lifetime
    pub fn update_system(assets: Res<Self>, mut events: ResMut<Events<AssetEvent<A>>>) {
        while let Some((id, asset)) = assets.receiver.try_receive() {
            if let Ok(asset) = asset.downcast() {
                match assets.store.insert(id, *asset) {
                    Some(_) => events.send(AssetEvent::Updated(AssetHandle::weak(id))),
                    None => events.send(AssetEvent::Created(AssetHandle::weak(id))),
                }
            }
        }
        // Ref Counter events
        while let Ok(e) = assets.ref_receiver.try_recv() {
            match e {
                RefEvent::Increase { id } => {
                    let mut count = assets.ref_count.entry(id).or_insert(0);
                    *count.value_mut() += 1
                }
                RefEvent::Decrease { id } => {
                    let mut count = assets
                        .ref_count
                        .get_mut(&id)
                        .expect("[Assets] (update_system) Decrease on non existing handle");
                    *count.value_mut() -= 1;
                    if *count.value() == 0u32 {
                        log::info!(
                            "Destroying Asset {:?} of Asset Type {}",
                            id,
                            type_name::<A>()
                        );
                        assets.store.remove(&id);
                    }
                }
            }
        }
    }
}
