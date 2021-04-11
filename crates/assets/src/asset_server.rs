use crate::{
    asset::Asset,
    assets::Assets,
    channels::{
        asset_pipe, AssetPipeReceiver, AssetPipeSender, AssetReceiverMap, AssetSenderMap,
        RefCounterMap,
    },
    file_spy::FileSpy,
    handle::{AssetHandle, AssetHandleUntyped, HandleId, LabelId},
    loader::{AssetLoader, LoadContext},
};
use core::anyhow::{self, Result};
use core::thiserror::{self, Error};

use ecs::prelude::Res;
use std::{
    any::TypeId,
    fs::File,
    io::Read,
    ops::Deref,
    path::{Path, PathBuf},
    sync::Arc,
};
use tasks::{futures::future, lock::RwLock, task_pool::TaskPool};

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

#[derive(Clone)]
pub struct AssetServer {
    task_pool: TaskPool,
    // Asset Pipes
    senders: AssetSenderMap,
    receivers: AssetReceiverMap,
    // Ref Counter Channels
    pub(crate) ref_counter: RefCounterMap,
    loaders: Arc<RwLock<Vec<Arc<dyn AssetLoader>>>>,
    /// the spy
    file_spy: Arc<FileSpy>,
}

impl AssetServer {
    pub fn new(pool: impl Deref<Target = TaskPool>) -> Self {
        Self {
            task_pool: pool.deref().clone(),
            senders: Default::default(),
            receivers: Default::default(),
            ref_counter: Default::default(),
            loaders: Default::default(),
            file_spy: Default::default(),
        }
    }

    pub fn register_asset<A: Asset>(&self) -> Assets<A> {
        let (tx, rx) = asset_pipe();
        self.senders.add_pipe::<A>(tx);
        self.receivers.add_pipe::<A>(rx.clone());
        self.ref_counter.add_pipe::<A>();
        Assets::new(rx, self.ref_counter.get_pipe::<A>().unwrap().1.clone())
    }

    pub async fn add_loader(&self, loader: impl AssetLoader + 'static) {
        let mut loaders = self.loaders.write().await;
        loaders.push(Arc::new(loader));
    }

    pub fn add_loader_sync(&self, loader: impl AssetLoader + 'static) {
        tasks::futures::future::block_on(self.add_loader(loader));
    }

    pub fn add_loaded_asset<A: Asset>(&self, label: impl AsRef<str>, asset: A) -> AssetHandle<A> {
        let id = HandleId::LabelId(label.into());
        let ref_pipe = self
            .ref_counter
            .get_pipe::<A>()
            .expect("[AssetServer] (add_loaded_asset) failed to get ref sender");
        let handle: AssetHandle<A> = AssetHandle::strong(id, ref_pipe.0.clone());
        future::block_on(self.senders.get_pipe::<A>().send((id, Box::new(asset))))
            .expect("[AssetServer] (add_loaded_asset) failed to send asset");
        handle
    }

    pub async fn update_asset<A: Asset>(&self, handle: &AssetHandle<A>, asset: A) {
        self.senders
            .get_pipe::<A>()
            .send((handle.id, Box::new(asset)))
            .await
            .expect(&format!(
                "[AssetServer] failed to update asset: {:?}",
                handle
            ))
    }

    pub fn load_asset<A: Asset>(&self, path: impl AsRef<str>) -> AssetHandle<A> {
        self.load_asset_untyped(path, TypeId::of::<A>()).typed()
    }

    pub fn load_asset_untyped(&self, path: impl AsRef<str>, type_id: TypeId) -> AssetHandleUntyped {
        let path_buf = {
            let mut buf = PathBuf::new(); // from(env!("CARGO_MANIFEST_DIR"));
            buf.push(path.as_ref());
            buf
        };

        self.file_spy.watch_asset(path_buf.clone());

        let id = self.load_internal(path_buf);
        let ref_pipe = self
            .ref_counter
            .get_pipe_from_type(type_id)
            .expect("[AssetServer] (add_loaded_asset) failed to get ref sender");
        AssetHandleUntyped::strong(id, ref_pipe.0.clone())
    }

    fn load_internal(&self, path: impl AsRef<Path> + Send + 'static) -> HandleId {
        let id = HandleId::from_path(path.as_ref());
        let server = self.clone();
        {
            let task = self.task_pool.spawn(async move {
                let path = path.as_ref();
                if let Err(e) = server.load_async(path, id).await {
                    log::error!("[AssetServer] (load_async) failed to load asset {:?}", path);
                    log::error!("{}", e);
                }
            });
            // AAAAAAAAnd then we don't care about it anymore
            task.detach();
        }
        id
    }

    async fn load_async(
        self,
        path: impl AsRef<Path>,
        handle: HandleId,
    ) -> Result<(), LoadAssetError> {
        // Get Fields
        let Self {
            loaders,
            senders,
            ref_counter,
            ..
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

        let load_context = LoadContext::new(senders, ref_counter, path, handle);
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
