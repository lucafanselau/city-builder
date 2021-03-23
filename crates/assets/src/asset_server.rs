use anyhow::{Context, Result};
use dashmap::{mapref::one::Ref, DashMap};
use std::{
    any::TypeId,
    collections::HashMap,
    ffi::OsStr,
    fs::File,
    io::Read,
    ops::Deref,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};
use tasks::task_pool::TaskPool;
use thiserror::Error;

use crate::{
    asset::{Asset, AssetChannel},
    assets::Assets,
    handle::{AssetHandle, AssetHandleId, AssetHandleUntyped},
    loader::{AssetLoader, LoadContext},
};

#[derive(Debug, Error)]
pub enum LoadAssetError {
    #[error("There is no registered loader for the extension {ext}")]
    NoLoader { ext: String },
    #[error("The Path {0} contains no extensions")]
    MissingExtension(String),
    #[error("The requested Asset could not be found")]
    NotFound(#[from] std::io::Error),
}

#[derive(Debug, Clone, Default)]
pub struct ChannelMap(Arc<DashMap<TypeId, AssetChannel>>);

impl ChannelMap {
    pub fn add_channel<A: Asset>(&self, channel: AssetChannel) {
        let type_id = std::any::TypeId::of::<A>();
        self.0.insert(type_id, channel);
    }

    pub fn get_channel<A: Asset>(&self) -> Ref<TypeId, AssetChannel> {
        let type_id = std::any::TypeId::of::<A>();
        self.0.get(&type_id).unwrap()
    }
}

#[derive(Clone)]
pub struct AssetServer {
    task_pool: TaskPool,
    pub channels: ChannelMap,
    // NOTE(luca): When the server is cloned, this vec contains the same loaders,
    // but when the user adds a loader after that, not all instances of the asset
    // server will be able to find that, not sure though if that is an actual problem
    loaders: Vec<Arc<Box<dyn AssetLoader>>>,
}

impl AssetServer {
    pub fn new(pool: impl Deref<Target = TaskPool>) -> Self {
        Self {
            task_pool: pool.deref().clone(),
            channels: Default::default(),
            loaders: Default::default(),
        }
    }

    pub fn register_asset<A: Asset>(&self) -> Assets<A> {
        let asset_channel = AssetChannel::new();
        self.channels.add_channel::<A>(asset_channel.clone());
        Assets::new(asset_channel)
    }

    pub fn add_loader(&mut self, loader: impl AssetLoader + 'static) {
        self.loaders.push(Arc::new(Box::new(loader)));
    }

    pub fn load_asset<A: Asset>(
        &self,
        path: impl Into<String>,
    ) -> Result<AssetHandle<A>, LoadAssetError> {
        self.load_asset_untyped(path).map(|h| h.typed())
    }

    pub fn load_asset_untyped(
        &self,
        path: impl Into<String>,
    ) -> Result<AssetHandleUntyped, LoadAssetError> {
        let path_buf = {
            let mut buf = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            buf.push(path.into());
            buf
        };

        let handle = AssetHandleUntyped::new(AssetHandleId::from_path(path_buf.as_path()));

        // First we will try to access the fs metedata (if this fails, the path is invalid or the asset does not exist)
        let metadata = std::fs::metadata(path_buf.as_path())?;
        // Next we will find the matching loader
        let extension = path_buf.as_path().extension().ok_or_else(|| {
            LoadAssetError::MissingExtension(path_buf.as_os_str().to_string_lossy().into())
        })?;
        let loader: &Arc<Box<dyn AssetLoader>> = self
            .loaders
            .iter()
            .find(|l| l.ext().contains(&extension.to_str().unwrap()))
            .ok_or_else(|| LoadAssetError::NoLoader {
                ext: extension.to_str().unwrap().into(),
            })?;

        let task = self.task_pool.spawn(self.clone().load_async(
            path_buf,
            handle.clone(),
            metadata,
            loader.clone(),
        ));

        task.detach();

        Ok(handle)
    }

    pub(crate) async fn load_async(
        self,
        path_buf: PathBuf,
        handle: AssetHandleUntyped,
        metadata: std::fs::Metadata,
        loader: Arc<Box<dyn AssetLoader>>,
    ) {
        let mut f = File::open(path_buf.clone()).expect("load asset");
        let mut bytes = vec![0; metadata.len() as usize];
        f.read_exact(&mut bytes).expect("buffer overflow");

        let load_context = LoadContext::new(self, path_buf.as_path(), handle);

        if let Err(e) = loader.load(&bytes, load_context).await {
            log::error!(
                "[AssetServer] (load_async) failed to load asset {:?}",
                path_buf.as_path()
            );
            log::error!("{}", e);
            // Although that might be a bit to much 😉
            panic!();
        }
    }
}
