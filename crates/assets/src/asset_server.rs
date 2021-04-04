use crate::{
    asset::{Asset, AssetChannel},
    assets::Assets,
    file_spy::FileSpy,
    handle::{AssetHandle, AssetHandleUntyped, HandleId},
    loader::{AssetLoader, LoadContext},
};
use core::anyhow::{self, Result};
use core::thiserror::{self, Error};
use dashmap::{mapref::one::Ref, DashMap};
use ecs::prelude::Res;
use std::{
    any::TypeId,
    fs::File,
    io::Read,
    ops::Deref,
    path::{Path, PathBuf},
    sync::Arc,
};
use tasks::{lock::RwLock, task_pool::TaskPool};

#[derive(Debug, Error)]
pub enum LoadAssetError {
    #[error("There is no registered loader for the extension {ext}")]
    NoLoader { ext: String },
    #[error("The Path {0} contains no extensions")]
    MissingExtension(String),
    #[error("The requested Asset could not be found")]
    NotFound(#[from] std::io::Error),
    #[error("Loader failed: {}", .0)]
    LoaderError(#[from] anyhow::Error),
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
    loaders: Arc<RwLock<Vec<Arc<dyn AssetLoader>>>>,
    /// the spy
    file_spy: Arc<FileSpy>,
}

impl AssetServer {
    pub fn new(pool: impl Deref<Target = TaskPool>) -> Self {
        Self {
            task_pool: pool.deref().clone(),
            channels: Default::default(),
            loaders: Default::default(),
            file_spy: Default::default(),
        }
    }

    pub fn register_asset<A: Asset>(&self) -> Assets<A> {
        let asset_channel = AssetChannel::new();
        self.channels.add_channel::<A>(asset_channel.clone());
        Assets::new(asset_channel)
    }

    pub async fn add_loader(&self, loader: impl AssetLoader + 'static) {
        let mut loaders = self.loaders.write().await;
        loaders.push(Arc::new(loader));
    }

    pub fn add_loader_sync(&self, loader: impl AssetLoader + 'static) {
        tasks::futures::future::block_on(self.add_loader(loader));
    }

    pub fn load_asset<A: Asset>(&self, path: impl AsRef<str>) -> AssetHandle<A> {
        self.load_asset_untyped(path).typed()
    }

    pub fn load_asset_untyped(&self, path: impl AsRef<str>) -> AssetHandleUntyped {
        let path_buf = {
            let mut buf = PathBuf::new(); // from(env!("CARGO_MANIFEST_DIR"));
            buf.push(path.as_ref());
            buf
        };

        self.file_spy.watch_asset(path_buf.clone());

        self.load_internal(path_buf)
    }

    fn load_internal(&self, path: impl AsRef<Path> + Send + 'static) -> AssetHandleUntyped {
        let handle = AssetHandleUntyped::new(HandleId::from_path(path.as_ref()));
        let server = self.clone();
        {
            let handle = handle.clone();
            let task = self.task_pool.spawn(async move {
                let path = path.as_ref();
                if let Err(e) = server.load_async(path, handle).await {
                    log::error!("[AssetServer] (load_async) failed to load asset {:?}", path);
                    log::error!("{}", e);
                }
            });
            // AAAAAAAAnd then we don't care about it anymore
            task.detach();
        }
        handle
    }

    async fn load_async(
        self,
        path: impl AsRef<Path>,
        handle: AssetHandleUntyped,
    ) -> Result<(), LoadAssetError> {
        // Get Fields
        let Self {
            loaders, channels, ..
        } = self;

        let path = path.as_ref();
        // First we will try to access the fs metedata (if this fails, the path is invalid or the asset does not exist)
        let metadata = std::fs::metadata(path)?;

        let mut f = File::open(&path)?;
        let mut bytes = vec![0; metadata.len() as usize];
        f.read_exact(&mut bytes)?;

        // Next we will find the matching loader
        let extension = path
            .extension()
            .map(|v| v.to_str())
            .flatten()
            .ok_or_else(|| LoadAssetError::MissingExtension(format!("{:?}", path)))?;
        let loaders = loaders.read().await;
        let loader = loaders
            .iter()
            .find(|l| l.ext().contains(&extension))
            .ok_or_else(|| LoadAssetError::NoLoader {
                ext: extension.into(),
            })?;

        let load_context = LoadContext::new(channels, path, handle);
        loader.load(&bytes, load_context).await?;

        Ok(())
    }

    pub fn update_system(server: Res<Self>) {
        while let Ok(event) = server.file_spy.rx().try_recv() {
            match event {
                Ok(event) => {
                    if let notify::Event {
                        kind: notify::EventKind::Modify(_),
                        paths,
                        ..
                    } = event
                    {
                        for path in paths {
                            // TODO: factor out
                            let path = path
                                .strip_prefix(std::env::current_dir().expect(
                                    "[AssetServer] (update_system) failed to get working dir",
                                ))
                                .expect("[AssetServer] (update_system) Failed to strip prefix")
                                .to_path_buf();
                            let _ = server.load_internal(path);
                        }
                    }
                }
                Err(e) => log::warn!("[AssetServer] (update_system) notify got an error: {}", e),
            }
        }
    }
}
