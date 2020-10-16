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
    (($($R:ident),*), ($($Q:ident),*,)) => {
        impl<Func, $($R: ResourceQuery,)* $($Q: HecsQuery,)*>
            IntoFunctionSystem<($($R,)*), ($($Q,)*)>
            for Func where Func: Fn($($R,)* $(QueryBorrow<$Q>,)*) +
                Fn(
                    $(<<$R as ResourceQuery>::Creator as ResourceCreator>::Item,)*
                    $(QueryBorrow<$Q>,)*) +
                Send + Sync +'static,
        {
            #[allow(non_snake_case)]
            #[allow(unused_variables)]
            fn into_system(self) -> Box<dyn System> {
                Box::new(FunctionSystem::new(
                    move |world: &World, resources: &Resources| {
                        let ($($R,)*) = resources.query::<($($R,)*)>().unwrap();
                        self($($R,)* $(world.query::<$Q>(),)*);
                    },
                    std::any::type_name::<Self>().into(),
                ))
            }
        }
    };
}

macro_rules! impl_into_systems {
    () => {
        impl_into_system!((), (,));
        impl_into_system!((Ra), (,));
        impl_into_system!((Ra, Rb), (,));
        impl_into_system!((Ra, Rb, Rc), (,));
        impl_into_system!((Ra, Rb, Rc, Rd), (,));
        impl_into_system!((Ra, Rb, Rc, Rd, Re), (,));
    };
    ($($Q:ident),*) => {
        impl_into_system!((), ($($Q,)*));
        impl_into_system!((Ra), ($($Q,)*));
        impl_into_system!((Ra, Rb), ($($Q,)*));
        impl_into_system!((Ra, Rb, Rc), ($($Q,)*));
        impl_into_system!((Ra, Rb, Rc, Rd), ($($Q,)*));
        impl_into_system!((Ra, Rb, Rc, Rd, Re), ($($Q,)*));
    };
}

impl_into_systems!();
impl_into_systems!(A);
impl_into_systems!(A, B);

// Same number of resources as in resource_query.rs

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

    fn no_resource_system(mut query: QueryBorrow<(&mut i32, &bool)>) {
        for (_e, (signed, _boolean)) in query.iter() {
            *signed *= 2;
        }
    }

    #[test]
    fn no_resource() {
        let (world, resources) = test_setup();

        let system = no_resource_system.into_system();
        system.run(&world, &resources);
        system.run(&world, &resources);

        let mut query = world.query::<&i32>();
        for (_e, signed) in query.iter() {
            assert_eq!(*signed, 4);
        }
    }
}
