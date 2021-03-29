use std::hash::BuildHasherDefault;

use crate::{
    asset::{Asset, AssetChannel},
    handle::AssetHandle,
    prelude::AssetEvent,
};
use core::anyhow::anyhow;
use dashmap::{mapref::one::Ref, DashMap};
use ecs::prelude::{Events, Res, ResMut};
use hash_hasher::{HashBuildHasher, HashHasher};

type Hasher = BuildHasherDefault<HashHasher>;

/// Simple Collection for Assets
pub struct Assets<A: Asset> {
    store: DashMap<AssetHandle<A>, A, Hasher>,
    channel: AssetChannel,
}

impl<A: Asset> Assets<A> {
    pub(crate) fn new(channel: AssetChannel) -> Self {
        Self {
            store: DashMap::with_hasher(HashBuildHasher::default()),
            channel,
        }
    }

    pub fn query<'a>(
        &'a self,
        handles: &[AssetHandle<A>],
    ) -> core::anyhow::Result<Vec<Ref<'a, AssetHandle<A>, A, Hasher>>> {
        let assets: Vec<Ref<AssetHandle<A>, A, BuildHasherDefault<HashHasher>>> =
            handles.iter().filter_map(|h| self.store.get(h)).collect();
        if assets.len() == handles.len() {
            Ok(assets)
        } else {
            Err(anyhow!("failed to query all handles"))
        }
    }

    pub fn try_get(&self, handle: &AssetHandle<A>) -> Option<Ref<AssetHandle<A>, A, Hasher>> {
        self.store.get(handle)
    }

    pub fn get(&self, handle: &AssetHandle<A>) -> Ref<AssetHandle<A>, A, Hasher> {
        self.try_get(handle)
            .expect("[Assets] failed to retrieve asset")
    }

    // TODO: Asset Lifetime
    pub fn update_system(assets: Res<Self>, mut events: ResMut<Events<AssetEvent<A>>>) {
        while let Some((id, a)) = assets.channel.try_receive::<A>() {
            match assets.store.insert(id.clone(), *a) {
                Some(_) => events.send(AssetEvent::Updated(id)),
                None => events.send(AssetEvent::Created(id)),
            }
        }
    }
}
