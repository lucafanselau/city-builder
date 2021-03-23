use crate::{
    asset::{Asset, AssetChannel},
    handle::AssetHandle,
};
use app::Res;
use dashmap::{mapref::one::Ref, DashMap};

/// Simple Collection for Assets
pub struct Assets<A: Asset> {
    store: DashMap<AssetHandle<A>, A>,
    channel: AssetChannel,
}

impl<A: Asset> Assets<A> {
    pub(crate) fn new(channel: AssetChannel) -> Self {
        Self {
            store: Default::default(),
            channel,
        }
    }

    pub fn query<'a>(&'a self, handles: &[AssetHandle<A>]) -> Vec<Ref<'a, AssetHandle<A>, A>> {
        handles.iter().map(|h| self.store.get(h).unwrap()).collect()
    }

    pub fn try_get(&self, handle: &AssetHandle<A>) -> Option<Ref<AssetHandle<A>, A>> {
        self.store.get(handle)
    }

    pub fn get(&self, handle: &AssetHandle<A>) -> Ref<AssetHandle<A>, A> {
        self.try_get(handle)
            .expect("[Assets] failed to retrieve asset")
    }

    // TODO: !! Events and Asset Destructuring
    pub fn update_system(assets: Res<Self>) {
        while let Some((id, a)) = unsafe { assets.channel.try_receive::<A>() } {
            assets.store.insert(id, *a);
        }
    }
}
