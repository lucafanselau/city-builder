//! Provides an App struct, which basically ties all the loose ends together
#![feature(trait_alias)]

pub mod stages;
pub mod timing;

pub use core;
pub use ecs::prelude::*;
use ecs::{resource::Resource, system::MutatingSystem};
use std::{any::type_name, borrow::Cow};
use tasks::{AsyncComputePool, ComputePool};
pub use timing::Timing;

pub use assets::prelude::*;

type Runner = Option<Box<dyn FnOnce(Resources, World, Scheduler)>>;

pub struct App {
    resources: Resources,
    world: World,
    scheduler: Scheduler,
    plugins: Vec<Box<dyn FnOnce(&mut Self)>>,
    runner: Runner,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

fn init_app(app: &mut App) {
    // First we need to insert the pools
    if !app.get_resources().contains::<ComputePool>() {
        app.insert_resource(ComputePool::default());
    }

    if !app.get_resources().contains::<AsyncComputePool>() {
        let pool = AsyncComputePool::default();
        app.insert_resource(pool.clone());
        // Aaaaand then an asset server
        app.insert_resource(AssetServer::new(pool));
        app.add_system(stages::UPDATE, AssetServer::update_system.into_system());
    }
}

impl App {
    pub fn new() -> Self {
        let mut scheduler = Scheduler::new();

        for stage in stages::STAGES.iter() {
            scheduler.add_stage(*stage);
        }

        let default_plugins: Vec<Box<dyn FnOnce(&mut Self)>> =
            vec![Box::new(init_app), Box::new(timing::init)];

        Self {
            resources: Resources::new(),
            world: World::new(),
            scheduler,
            // Add default plugins
            plugins: default_plugins,
            runner: None,
        }
    }

    pub fn get_resources(&mut self) -> &mut Resources {
        &mut self.resources
    }

    pub fn get_world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    pub fn insert_resource<T: Resource>(&mut self, initial: T) {
        self.get_resources().insert(initial).unwrap_or_else(|e| {
            let name = type_name::<T>();
            panic!(
                "[App] (insert_resource) error occurred while inserting type [{}]: {}",
                name, e
            );
        })
    }

    pub fn get_res<T: Resource>(&self) -> Res<T> {
        self.resources.get::<T>().unwrap_or_else(|e| {
            let name = type_name::<T>();
            panic!(
                "[App] (get_res) error occurred while resource query for type [{}]: {}",
                name, e
            );
        })
    }

    pub fn add_plugin<Func>(&mut self, system: Func)
    where
        Func: 'static + FnOnce(&mut Self),
    {
        self.plugins.push(Box::new(system));
    }

    pub fn add_system(&mut self, stage: impl Into<Cow<'static, str>>, system: Box<dyn System>) {
        self.scheduler.add_system_to_stage(stage, system);
    }

    pub fn add_mut_system(&mut self, system: Box<dyn MutatingSystem>) {
        self.scheduler.add_mut_system(system)
    }

    pub fn add_event<T: Event>(&mut self) {
        self.resources
            .insert::<Events<T>>(Events::new())
            .expect("[App] failed to insert event");

        self.scheduler.add_system_to_stage(
            stages::UPDATE_EVENTS,
            Events::<T>::update_system.into_system(),
        );
    }

    pub fn load_asset<A: Asset>(
        &self,
        path: impl AsRef<str>,
    ) -> core::anyhow::Result<AssetHandle<A>> {
        let server = self.resources.get::<AssetServer>()?;
        let handle = server.load_asset(path);
        Ok(handle)
    }

    pub fn add_asset_loader(&self, loader: impl AssetLoader + 'static) {
        let server = self
            .resources
            .get_mut::<AssetServer>()
            .expect("[App] (add_asset_loader) failed to get asser_server");
        server.add_loader_sync(loader);
    }

    pub fn register_asset<A: Asset>(&mut self) {
        let assets = {
            let server = self
                .resources
                .get::<AssetServer>()
                .expect("[App] (register_asset) failed to get asset_server");
            server.register_asset::<A>()
        };
        self.resources
            .insert(assets)
            .expect("[App] (register_asset) failed to insert assets");

        // NOTE(luca): not quite sure if the stage is the best
        self.scheduler.add_system_to_stage(
            stages::PREPARE_FRAME,
            Assets::<A>::update_system.into_system(),
        );

        self.add_event::<AssetEvent<A>>();
    }

    pub fn set_runner<Func>(&mut self, runner: Func)
    where
        Func: 'static + FnOnce(Resources, World, Scheduler),
    {
        self.runner = Some(Box::new(runner));
    }

    pub fn run(mut self) {
        // Run Plugins
        let mut startup: Vec<Box<dyn FnOnce(&mut Self)>> = self.plugins.drain(..).collect();
        for start_system in startup.drain(..) {
            start_system(&mut self);
        }

        // Expect runner
        let runner = match self.runner {
            Some(runner) => runner,
            None => panic!("[App] (run) there is no runner specified, make sure set_runner is called at least once!")
        };

        // Run the App
        runner(self.resources, self.world, self.scheduler);
    }
}
