// We are copying bevy a bit here (from the layout and abstraction idea)
use crate::system::system::FunctionSystem;
use crate::{
    resource::{ResourceCreator, ResourceQuery, Resources},
    system::system::System,
};
use hecs::{Query as HecsQuery, QueryBorrow, World};

pub trait IntoFunctionSystem<Resources: ResourceQuery, Q: HecsQuery> {
    fn into_system(self) -> Box<dyn System>;
}

#[allow(unused_macros)]
macro_rules! impl_into_system {
    ($($R:ident),*) => {
        impl<Func, $($R: ResourceQuery,)* Query: HecsQuery>
            IntoFunctionSystem<($($R,)*), Query>
            for Func where Func: Fn($($R,)* QueryBorrow<Query>) +
                Fn(
                    $(<<$R as ResourceQuery>::Creator as ResourceCreator>::Item,)*
                    QueryBorrow<Query>) +
                Send + Sync +'static,
        {
            #[allow(non_snake_case)]
            fn into_system(self) -> Box<dyn System> {
                Box::new(FunctionSystem::new(
                    move |world: &World, resources: &Resources| {
                        let ($($R,)*) = resources.query::<($($R,)*)>().unwrap();
                        self($($R,)* world.query());
                    },
                    std::any::type_name::<Self>().into(),
                ))
            }
        }
    };
}

// Same number of resources as in resource_query.rs
impl_into_system!(Ra);
impl_into_system!(Ra, Rb);
impl_into_system!(Ra, Rb, Rc);
impl_into_system!(Ra, Rb, Rc, Rd);
impl_into_system!(Ra, Rb, Rc, Rd, Re);

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefMut;

    fn test_setup() -> (World, Resources) {
        let mut world = World::new();
        world.spawn((1i32, true));
        world.spawn((1i32, false));
        let mut resources = Resources::new();
        resources.insert(0i32).unwrap();
        (world, resources)
    }

    fn my_first_system(mut counter: RefMut<i32>, mut query: QueryBorrow<(&mut i32, &bool)>) {
        *counter += 1;
        for (_e, (signed, boolean)) in query.iter() {
            if *boolean {
                *signed *= 2
            };
        }
    }

    #[test]
    fn run_system() {
        let (world, resources) = test_setup();

        let system = my_first_system.into_system();
        system.run(&world, &resources);
        system.run(&world, &resources);

        let counter = resources.get::<i32>().unwrap();
        assert_eq!(*counter, 2);

        let mut query = world.query::<(&i32, &bool)>();
        for (_e, (signed, boolean)) in query.iter() {
            if *boolean {
                assert_eq!(*signed, 4)
            } else {
                assert_eq!(*signed, 1)
            }
        }
    }
}
