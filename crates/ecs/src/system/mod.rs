pub mod into_system;

use crate::resource::Resources;
use hecs::World;
use std::borrow::Cow;

pub trait System {
    fn name(&self) -> Cow<'static, str>;
    fn run(&self, world: &hecs::World, resources: &Resources);
}

pub trait FunctionSystemCallback = Fn(&hecs::World, &Resources) + 'static;

pub struct FunctionSystem<Func>
where
    Func: FunctionSystemCallback,
{
    callback: Func,
    name: Cow<'static, str>,
}

impl<Func: FunctionSystemCallback> FunctionSystem<Func> {
    // TODO: Remove
    #[allow(dead_code)]
    pub fn new(func: Func, name: Cow<'static, str>) -> Self {
        FunctionSystem {
            callback: func,
            name: name.clone(),
        }
    }
}

impl<Func: FunctionSystemCallback> System for FunctionSystem<Func> {
    fn name(&self) -> Cow<'static, str> {
        self.name.clone()
    }

    fn run(&self, world: &World, resources: &Resources) {
        (self.callback)(world, resources);
    }
}

pub trait MutatingSystem {
    fn name(&self) -> Cow<'static, str>;
    fn run(&mut self, world: &mut hecs::World, resources: &mut Resources);
}

pub trait MutatingFunctionSystemCallback = FnMut(&mut hecs::World, &mut Resources) + 'static;

pub struct MutatingFunctionSystem<Func>
where
    Func: MutatingFunctionSystemCallback,
{
    pub(crate) callback: Func,
    pub(crate) name: Cow<'static, str>,
}

impl<Func: MutatingFunctionSystemCallback> MutatingFunctionSystem<Func> {
    // TODO: Remove
    #[allow(dead_code)]
    pub fn new(func: Func, name: Cow<'static, str>) -> Self {
        MutatingFunctionSystem {
            callback: func,
            name: name.clone(),
        }
    }
}

impl<Func: MutatingFunctionSystemCallback> MutatingSystem for MutatingFunctionSystem<Func> {
    fn name(&self) -> Cow<'static, str> {
        self.name.clone()
    }

    fn run(&mut self, world: &mut World, resources: &mut Resources) {
        (self.callback)(world, resources);
    }
}
