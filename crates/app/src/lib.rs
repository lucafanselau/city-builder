//! Provides an App struct, which basically ties all the loose ends together

pub mod stages;

pub use ecs::prelude::*;
use std::borrow::Cow;

pub struct App {
    world: World,
    resources: Resources,
    scheduler: Scheduler,
    plugins: Vec<Box<dyn FnOnce(&mut Self)>>,
    runner: Option<Box<dyn FnOnce(World, Resources, Scheduler)>>,
}

impl App {
    pub fn new() -> Self {
        let mut scheduler = Scheduler::new();

        for stage in stages::STAGES.iter() {
            scheduler.add_stage(stage.clone());
        }

        Self {
            world: World::new(),
            resources: Resources::new(),
            scheduler,
            plugins: Vec::new(),
            runner: None,
        }
    }

    pub fn get_resources(&mut self) -> &mut Resources {
        &mut self.resources
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

    pub fn set_runner<Func>(&mut self, runner: Func)
    where
        Func: 'static + FnOnce(World, Resources, Scheduler),
    {
        self.runner = Some(Box::new(runner));
    }

    pub fn run(mut self) {
        let mut startup: Vec<Box<dyn FnOnce(&mut Self)>> = self.plugins.drain(..).collect();
        for start_system in startup.drain(..) {
            start_system(&mut self);
        }

        let runner = match self.runner {
            Some(runner) => runner,
            None => panic!("[App] (run) there is no runner specified, make sure set_runner is called at least once!")
        };

        runner(self.world, self.resources, self.scheduler);
    }
}