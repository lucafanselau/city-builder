// Used to reduce code repetition
// if there are drawbacks, we might need to remove that
#![feature(trait_alias)]

mod into_system;
mod resource;
mod system;

// This whole thing is largely based on the bevy_ecs, since it seems to be quite a good ecs.
// maybe it would be easier to just pull in the bevy_ecs, but i guess here we are
