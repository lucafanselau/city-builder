#![feature(trait_alias)]

mod asset;
mod asset_server;
mod assets;
mod def;
mod handle;
mod loader;

use app::App;
pub use def::*;
use tasks::*;

pub fn init(app: &mut App) {
    if !app.get_resources().contains::<ComputePool>() {
        app.insert_resource(ComputePool::default());
    }

    if !app.get_resources().contains::<AsyncComputePool>() {
        app.insert_resource(AsyncComputePool::default());
    }

    // Now that this is out of the way, we can create an asset server
}

#[cfg(test)]
mod tests {
    use std::{
        cell::RefCell,
        path::{Path, PathBuf},
        thread,
        time::Duration,
    };

    use assets::Assets;

    use crate::loader::AssetLoader;

    use super::*;
    struct SimpleAsset {
        number: i32,
    }

    struct SimpleLoader {}

    impl AssetLoader for SimpleLoader {
        fn load<'a>(
            &'a self,
            bytes: &'a [u8],
            ctx: loader::LoadContext<'a>,
        ) -> BoxedFuture<'a, anyhow::Result<()>> {
            Box::pin(async move {
                // We expect .dat files to be utf-8 encoded
                let input = String::from_utf8_lossy(bytes);
                // now we parse it to an intn
                let number = input.trim().parse::<i32>()?;
                ctx.server
                    .channels
                    .get_channel::<SimpleAsset>()
                    .send(ctx.handle.typed(), SimpleAsset { number })
                    .await;
                Ok(())
            })
        }

        fn ext(&self) -> &[&str] {
            &["dat"]
        }
    }

    #[test]
    fn use_case() {
        let task_pool = tasks::ComputePool::default();

        let mut server = asset_server::AssetServer::new(task_pool);
        // NOTE(luca): We need to use ref cell here so simulate the resource system
        // (although that should probably also not rely on refcell 😟)
        let simple_assets = RefCell::new(server.register_asset::<SimpleAsset>());
        server.add_loader(SimpleLoader {});

        // println!("{:?}", path_buf);
        let handle = match server.load_asset("../../assets/file.dat") {
            Ok(h) => h,
            Err(e) => {
                println!("failed to load file.dat, with error: {}", e);
                panic!()
            }
        };

        while simple_assets.borrow().try_get(&handle).is_none() {
            Assets::<SimpleAsset>::update_system(simple_assets.borrow());
            thread::sleep(Duration::from_millis(200u64))
        }
        // And now we check if everything is correct
        {
            let simple_assets = simple_assets.borrow();
            let my_asset = simple_assets.try_get(&handle);
            assert!(my_asset.is_some());
            assert_eq!(my_asset.unwrap().number, 1023);
        }
    }
}
