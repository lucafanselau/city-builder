// Used to reduce code repetition
// if there are drawbacks, we might need to remove that
#![feature(trait_alias)]

pub mod event;
pub mod resource;
pub mod schedule;
pub mod system;

pub mod prelude {
    pub use crate::event::{Event, Events};
    pub use crate::resource::{ResourceQuery, Resources};
    pub use crate::schedule::{executor::SequentialExecutor, scheduler::Scheduler};
    pub use crate::system::{
        into_system::{IntoFunctionSystem, IntoMutatingSystem},
        System,
    };
    pub use hecs::{QueryBorrow, World};
    // Our resource ref type (should be replace sometime)
    pub use std::cell::{Ref as Res, RefMut as ResMut};
}

// This whole thing is largely based on the bevy_ecs, since it seems to be quite a good ecs.
// maybe it would be easier to just pull in the bevy_ecs, but i guess here we are
