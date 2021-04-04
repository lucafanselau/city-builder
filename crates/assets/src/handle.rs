use core::HASHER;
use std::{
    hash::{Hash, Hasher},
    path::Path,
};

use crate::{asset::Asset, path::AssetPath};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HandleId {
    AssetPathId(AssetPathId),
}

impl From<AssetPathId> for HandleId {
    fn from(id: AssetPathId) -> Self {
        Self::AssetPathId(id)
    }
}

impl HandleId {
    pub fn from_asset_path(path: AssetPath) -> Self {
        Self::AssetPathId(path.into())
    }

    pub fn from_path(path: &Path) -> Self {
        Self::AssetPathId(AssetPath::from_path(path).into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssetPathId {
    path_id: u64,
    label_id: u64,
}

impl<'a> From<AssetPath<'a>> for AssetPathId {
    fn from(path: AssetPath) -> Self {
        let mut hasher = HASHER.clone();
        path.path().hash(&mut hasher);
        let path_id = hasher.finish();
        path.label().hash(&mut hasher);
        let label_id = hasher.finish();
        Self { path_id, label_id }
    }
}

impl AssetPathId {
    pub fn from_path(path: AssetPath) -> Self {
        Self::from(path)
    }
}

#[derive(Debug)]
pub struct AssetHandle<A: Asset> {
    id: HandleId,
    _marker: std::marker::PhantomData<A>,
}

// region: manual eq, partial_eq and hash implementations for AssetHandle
impl<A: Asset> Clone for AssetHandle<A> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            _marker: Default::default(),
        }
    }
}
impl<A: Asset> PartialEq for AssetHandle<A> {
    fn eq(&self, other: &Self) -> bool {
        self.id.eq(&other.id)
    }
}
impl<A: Asset> Eq for AssetHandle<A> {}
impl<A: Asset> Hash for AssetHandle<A> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state)
    }
}

impl<A: Asset> AssetHandle<A> {
    pub fn untyped(self) -> AssetHandleUntyped {
        AssetHandleUntyped { id: self.id }
    }
}

#[derive(Debug, Clone)]
pub struct AssetHandleUntyped {
    id: HandleId,
}

impl AssetHandleUntyped {
    pub fn new(id: HandleId) -> Self {
        Self { id }
    }

    pub fn typed<A: Asset>(self) -> AssetHandle<A> {
        AssetHandle {
            id: self.id,
            _marker: Default::default(),
        }
    }
}
