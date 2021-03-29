use core::HASHER;
use std::{
    hash::{Hash, Hasher},
    path::Path,
};

use crate::asset::Asset;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AssetHandleId {
    PathId(AssetPathId),
}

impl From<AssetPathId> for AssetHandleId {
    fn from(id: AssetPathId) -> Self {
        Self::PathId(id)
    }
}

impl AssetHandleId {
    pub fn from_path(path: &Path) -> Self {
        Self::PathId(path.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssetPathId {
    path_id: u64,
}

impl From<&Path> for AssetPathId {
    fn from(path: &Path) -> Self {
        let mut hasher = HASHER.clone();
        path.hash(&mut hasher);
        Self {
            path_id: hasher.finish(),
        }
    }
}

impl AssetPathId {
    pub fn from_path(path: &Path) -> Self {
        Self::from(path)
    }
}

#[derive(Debug)]
pub struct AssetHandle<A: Asset> {
    id: AssetHandleId,
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
    id: AssetHandleId,
}

impl AssetHandleUntyped {
    pub fn new(id: AssetHandleId) -> Self {
        Self { id }
    }

    pub fn typed<A: Asset>(self) -> AssetHandle<A> {
        AssetHandle {
            id: self.id,
            _marker: Default::default(),
        }
    }
}
