//! Provides an App struct, which basically ties all the loose ends together
#![feature(trait_alias)]

pub mod event;
pub mod stages;
pub mod timing;

pub use ecs::prelude::*;
use ecs::{resource::Resource, system::MutatingSystem};
use event::{Event, Events};
use std::{any::type_name, borrow::Cow, cell::Ref};
pub use timing::Timing;

type Runner = Option<Box<dyn FnOnce(World, Resources, Scheduler)>>;

pub struct App {
    world: World,
    resources: Resources,
    scheduler: Scheduler,
    plugins: Vec<Box<dyn FnOnce(&mut Self)>>,
    runner: Runner,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        let mut scheduler = Scheduler::new();

        for stage in stages::STAGES.iter() {
            scheduler.add_stage(*stage);
        }

        let default_plugins: Vec<Box<dyn FnOnce(&mut Self)>> = vec![Box::new(timing::init)];

        Self {
            world: World::new(),
            resources: Resources::new(),
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

    pub fn get_res<T: Resource>(&self) -> Ref<T> {
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

    pub fn set_runner<Func>(&mut self, runner: Func)
    where
        Func: 'static + FnOnce(World, Resources, Scheduler),
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
        runner(self.world, self.resources, self.scheduler);
    }
}
