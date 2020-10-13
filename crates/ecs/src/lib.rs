// Used to reduce code repetition
// if there are drawbacks, we might need to remove that
#![feature(trait_alias)]

mod into_system;
mod resources;
mod system;

use crate::into_system::IntoFunctionSystem;
use crate::system::{FunctionSystem, System};
use hecs::*;
use log::*;
use std::borrow::Cow;

struct Position {
    x: f32,
    y: f32,
}
struct Velocity {
    dx: f32,
    dy: f32,
}

pub fn test_the_shit() {
    // quasi main function for testing purposes

    let mut world = World::new();

    let player = world.spawn((Position { x: 0., y: 0. }, Velocity { dx: 1.0, dy: -1.0 }));

    let mut query = world.query::<(&mut Position, &Velocity)>();
    for (_e, (p, v)) in query.iter() {
        p.x += v.dx;
        p.y += v.dy; /* * dt */
    }

    // for archetype in world.archetypes() {
    //     debug!("found archetype: {:?}", archetype);
    // }
}

fn entity_bundle(x: f32, y: f32, dx: f32, dy: f32) -> (Position, Velocity) {
    (Position { x, y }, Velocity { dx, dy })
}

/*fn my_first_system(mut query: hecs::QueryBorrow<(&mut Position, &Velocity)>) {
    for (_e, (p, v)) in query.iter() {
        p.x += v.dx;
        p.y += v.dy;
    }
}

impl<Func> IntoFunctionSystem for Func
where
    Func: Fn(hecs::QueryBorrow<(&mut Position, &Velocity)>) + 'static,
{
    fn into_system(self) -> Box<dyn System> {
        Box::new(FunctionSystem::new(
            move |world, resources| self(world.query()),
            Cow::from("my_first_system"),
        ))
    }
}*/

#[cfg(test)]
mod tests {
    use crate::{entity_bundle, Position, Velocity};

    #[test]
    fn add_system() {
        // First Mile Stone
        // Create a System
        let mut world = hecs::World::new();

        let a = world.spawn(entity_bundle(0.0, 0.0, 1.0, 1.0));
        let b = world.spawn(entity_bundle(-5.0, 2.0, -1.0, -1.0));

        use crate::into_system::IntoFunctionSystem;
        // let my_system = my_first_system.into_system();

        // my_system.run(&world);

        let position_a = world
            .get::<Position>(a)
            .expect("failed to get position of a");
        assert_eq!(position_a.x, 1.0);
        assert_eq!(position_a.y, 1.0);

        let position_b = world
            .get::<Position>(b)
            .expect("failed to get position of b");
        assert_eq!(position_b.x, -6.0);
        assert_eq!(position_b.y, 1.0);
    }

    #[test]
    fn optional_component() {
        let mut world = hecs::World::new();

        let a = world.spawn(
            hecs::EntityBuilder::new()
                .add(Position { x: 1.2, y: 3.6 })
                .build(),
        );
        let b = world.spawn((Position { x: 1.2, y: 3.6 }, true));

        assert_eq!(
            world.query::<(&Position, Option<&bool>)>().iter().count(),
            2
        );
    }

    struct ExecutionCounter(u32);

    use std::any::Any;
    use std::borrow::Cow;
    use std::cell::{Ref, RefMut};

    fn second_system(
        exec: RefMut<ExecutionCounter>,
        mut query: hecs::QueryBorrow<(&mut Position, &Velocity)>,
    ) {
        // first increase the execution counter
        exec.0 += 1;
        // Then iterate over entities
        for (_e, (p, v)) in query.iter() {
            p.x += v.dx;
            p.y += v.dy;
        }
    }

    impl<Func, Resource, Query> super::IntoFunctionSystem<Resource, Query> for Func
    where
        Func: Fn(RefMut<Resource>, Query) + Fn(Resource, Query) + 'static,
        Resource: Any + 'static,
        Query: hecs::Query,
    {
        fn into_system(self) -> Box<dyn super::System> {
            Box::new(super::FunctionSystem::new(
                move |world| self(world.query()),
                Cow::from("my_first_system"),
            ))
        }
    }

    impl<Func, Resource, Rb, Query> super::IntoFunctionSystem<(Resource, Rb), Query> for Func
    where
        Func: Fn(RefMut<Resource>, Ref<Rb>, Query) + Fn(Resource, Rb, Query) + 'static,
        Resource: Any + 'static,
        Rb: Any + 'static,
        Query: hecs::Query,
    {
        fn into_system(self) -> Box<dyn super::System> {
            Box::new(super::FunctionSystem::new(
                move |world| self(world.query()),
                Cow::from("my_first_system"),
            ))
        }
    }

    #[test]
    fn a_resource_system() {
        let mut resources = crate::resources::Resources::new();
        // First we push back a resource
        resources.insert(ExecutionCounter(0));
    }
}
