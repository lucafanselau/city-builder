use std::{
    any::Any,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use app::{core::anyhow, Resources, World};

use crate::{prelude::GpuContext, resource::pipeline::Viewport};

pub struct FrameData<'a, Context: GpuContext> {
    pub cmd: &'a mut <Context as GpuContext>::CommandEncoder,
    pub frame_index: u32,
    pub viewport: Viewport,
}

pub type PassReturn = Option<Box<dyn Any>>;

pub trait InitCallback<Context: GpuContext, U> =
    Fn(Arc<<Context as GpuContext>::RenderPassHandle>) -> Box<U>;
pub trait PassCallback<Context: GpuContext, U> =
    FnMut(FrameData<'_, Context>, &mut U, &World, &Resources) -> anyhow::Result<PassReturn>;
pub trait UserData = Send + Sync + 'static;

pub trait PassCallbacks<Context: GpuContext> {
    fn init(&mut self, render_pass: Arc<<Context as GpuContext>::RenderPassHandle>);
    fn run(
        &mut self,
        data: FrameData<Context>,
        world: &World,
        resources: &Resources,
    ) -> anyhow::Result<PassReturn>;
}

pub struct PassCallbacksImpl<Context: GpuContext, U: UserData + ?Sized> {
    init: Box<dyn InitCallback<Context, U>>,
    runner: Box<dyn PassCallback<Context, U>>,
    user_data: Option<Box<U>>,
}

impl<Context: GpuContext, U: UserData> PassCallbacksImpl<Context, U> {
    pub(crate) fn create(
        init: Box<dyn InitCallback<Context, U>>,
        runner: Box<dyn PassCallback<Context, U>>,
    ) -> Self {
        Self {
            init,
            runner,
            user_data: None,
        }
    }
}

impl<Context: GpuContext, U: UserData> PassCallbacks<Context> for PassCallbacksImpl<Context, U> {
    fn init(&mut self, render_pass: Arc<Context::RenderPassHandle>) {
        self.user_data = Some(self.init.deref()(render_pass));
    }

    fn run(
        &mut self,
        frame_data: FrameData<Context>,
        world: &World,
        resources: &Resources,
    ) -> anyhow::Result<PassReturn> {
        let data = self
            .user_data
            .as_mut()
            .expect("[PassCallbacks] no user data, did u call init for this pass?");
        self.runner.deref_mut()(frame_data, data, world, resources)
    }
}
