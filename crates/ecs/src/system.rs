use std::borrow::Cow;
use hecs;
use hecs::World;

pub trait System {
    fn name(&self) -> Cow<'static, str>;
    fn run(&self, world: &hecs::World);
}

pub struct FunctionSystem<Func> where Func: Fn(&hecs::World) + 'static {
    callback: Func,
    name: Cow<'static, str>
}

impl<Func: Fn(&hecs::World) + 'static> FunctionSystem<Func> {
    pub fn new(func: Func, name: Cow<'static, str>) -> Self {
        FunctionSystem {
            callback: func,
            name: name.clone()
        }
    }
}

impl <Func: Fn(&hecs::World) + 'static> System for FunctionSystem<Func> {
    fn name(&self) -> Cow<'static, str> {
        self.name.clone()
    }

    fn run(&self, world: &World) {
        (self.callback)(world)
    }
}

