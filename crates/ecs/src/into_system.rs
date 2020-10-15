// We are copying bevy a bit here (from the layout and abstraction idea)

use crate::system::System;

pub trait IntoFunctionSystem<Resources, Q> {
    fn into_system(self) -> Box<dyn System>;
}

// Now all the impls
// have a look into macros

// So example impls
/*
impl<Func, Resource, Query> IntoFunctionSystem<Resource, Query> for Func
    where
        Resource: ResourceRef + Sized + 'static,
        Query: hecs::Query,
        Func: Fn(Resource, QueryBorrow<Query>) + 'static,
    {
        fn into_system(self) -> Option<Box<dyn super::System>> {
            None
        }
    }

    impl<Func, Resource, Rb, HecsQuery> IntoFunctionSystem<(Resource, Rb), HecsQuery> for Func
    where
        Resource: ResourceRef + Sized + 'static,
        Rb: ResourceRef + Sized + 'static,
        HecsQuery: hecs::Query,
        Func: Fn(<Resource as ResourceRef>::Item, <Rb as ResourceRef>::Item, QueryBorrow<HecsQuery>)
            + Fn(Resource, Rb, QueryBorrow<HecsQuery>)
            + 'static,
    {
        fn into_system(self) -> Option<Box<dyn super::System>> {
            None
        }
    }
 */
