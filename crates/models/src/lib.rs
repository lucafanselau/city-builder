#![feature(str_split_once)]

mod factory;
mod loader;

use app::{App, AssetServer};
use artisan::renderer::ActiveContext;
use core::log;
use loader::GltfLoader;
use std::sync::Arc;

pub fn init_models(app: &mut App) {
    if !app.get_resources().contains::<AssetServer>() {
        log::error!("[models] (init) no asset server");
        return;
    }

    if !app.get_resources().contains::<Arc<ActiveContext>>() {
        log::error!("[models] (init) no active context found");
        return;
    }
    let ctx = app.get_res::<Arc<ActiveContext>>().clone();
    app.add_asset_loader(GltfLoader { ctx });
}
