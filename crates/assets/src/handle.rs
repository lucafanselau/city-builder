use core::HASHER;
use std::{
    fmt::Debug,
    hash::{Hash, Hasher},
    path::Path,
};

use crate::{
    asset::Asset,
    channels::{RefEvent, RefSender},
    path::AssetPath,
    prelude::AssetServer,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub enum HandleId {
    AssetPathId(AssetPathId),
    LabelId(LabelId),
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub struct LabelId(u64);

impl<'a, T: AsRef<str>> From<T> for LabelId {
    fn from(label: T) -> Self {
        let mut hasher = HASHER.clone();
        label.as_ref().hash(&mut hasher);
        Self(hasher.finish())
    }
}

impl LabelId {
    pub fn from_label<T: AsRef<str>>(label: T) -> Self {
        Self::from(label)
    }
}

#[derive(Debug)]
pub enum HandleType {
    Strong(RefSender),
    Weak,
}

pub struct AssetHandle<A: Asset> {
    pub(crate) id: HandleId,
    pub(crate) handle_type: HandleType,
    pub(crate) _marker: std::marker::PhantomData<A>,
}

/// region: manual eq, partial_eq and hash implementations for AssetHandle
impl<A: Asset> Debug for AssetHandle<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AssetHandle")
            .field("id", &self.id)
            .field("handle_type", &self.handle_type)
            .finish()
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
    pub fn weak(id: HandleId) -> Self {
        Self {
            id,
            handle_type: HandleType::Weak,
            _marker: Default::default(),
        }
    }

    pub fn strong(id: HandleId, sender: RefSender) -> Self {
        sender
            .try_send(RefEvent::Increase { id })
            .expect("[AssetHandle] (strong) failed to send increase");
        Self {
            id,
            handle_type: HandleType::Strong(sender),
            _marker: Default::default(),
        }
    }

    pub fn is_strong(&self) -> bool {
        matches!(self.handle_type, HandleType::Strong(_))
    }

    pub fn is_weak(&self) -> bool {
        matches!(self.handle_type, HandleType::Weak)
    }

    pub fn make_strong(&mut self, server: &AssetServer) {
        if !self.is_strong() {
            let ref_pipe = server
                .ref_counter
                .get_pipe::<A>()
                .expect("[AssetHandle] (make_strong) failed to get sender");
            let sender = ref_pipe.0.clone();
            sender.try_send(RefEvent::Increase { id: self.id });
            self.handle_type = HandleType::Strong(sender);
        }
    }

    pub fn as_weak(&self) -> AssetHandle<A> {
        AssetHandle {
            id: self.id,
            handle_type: HandleType::Weak,
            _marker: Default::default(),
        }
    }

    /// Clones a strong handle
    ///
    /// Returns None if the Handle is not strong
    pub fn clone_strong(&self) -> Option<Self> {
        match &self.handle_type {
            HandleType::Strong(sender) => {
                let sender = sender.clone();
                sender
                    .try_send(RefEvent::Increase { id: self.id })
                    .expect("[AssetHandle] (clone_strong) failed to send ref event");
                Some(Self {
                    id: self.id,
                    handle_type: HandleType::Strong(sender),
                    _marker: Default::default(),
                })
            }
            HandleType::Weak => None,
        }
    }
}

impl<A: Asset> Drop for AssetHandle<A> {
    fn drop(&mut self) {
        if let HandleType::Strong(sender) = &self.handle_type {
            let _ = sender.try_send(RefEvent::Decrease { id: self.id });
        }
    }
}

#[derive(Debug)]
pub struct AssetHandleUntyped {
    pub(crate) id: HandleId,
    pub(crate) handle_type: HandleType,
}

impl AssetHandleUntyped {
    pub fn weak(id: HandleId) -> Self {
        Self {
            id,
            handle_type: HandleType::Weak,
        }
    }

    pub fn strong(id: HandleId, sender: RefSender) -> Self {
        sender
            .try_send(RefEvent::Increase { id })
            .expect("[AssetHandle] (strong) failed to send increase");
        Self {
            id,
            handle_type: HandleType::Strong(sender),
        }
    }

    pub fn is_strong(&self) -> bool {
        matches!(self.handle_type, HandleType::Strong(_))
    }

    pub fn is_weak(&self) -> bool {
        matches!(self.handle_type, HandleType::Weak)
    }

    pub fn typed<A: Asset>(self) -> AssetHandle<A> {
        match &self.handle_type {
            HandleType::Strong(sender) => {
                // first we will need to send an increase event, so that the drop function of self will not destroy the asset
                sender
                    .try_send(RefEvent::Increase { id: self.id })
                    .expect("[AssetHandleUntyped] (typed) failed to send ref event");

                AssetHandle::strong(self.id, sender.clone())
            }
            HandleType::Weak => AssetHandle::weak(self.id),
        }
    }
}

impl Drop for AssetHandleUntyped {
    fn drop(&mut self) {
        if let HandleType::Strong(sender) = &self.handle_type {
            let _ = sender.try_send(RefEvent::Decrease { id: self.id });
        }
    }
}
