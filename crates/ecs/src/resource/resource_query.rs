use crate::resource::{Resource, Resources};
use std::cell::{Ref, RefMut};
use std::marker::PhantomData;

pub trait ResourceQuery {
    type Creator: for<'a> ResourceCreator<'a>;
}

pub trait ResourceCreator<'a> {
    type Item;

    fn create(resources: &'a Resources) -> Option<Self::Item>;
}

impl<'a, R> ResourceQuery for Ref<'a, R>
where
    R: Resource,
{
    type Creator = ImmutableResourceCreator<R>;
}

pub struct ImmutableResourceCreator<R: Resource>(PhantomData<R>);

impl<'a, R: Resource> ResourceCreator<'a> for ImmutableResourceCreator<R> {
    type Item = Ref<'a, R>;

    fn create(resources: &'a Resources) -> Option<Self::Item> {
        resources.get::<R>().ok()
    }
}

impl<'a, R: Resource> ResourceQuery for RefMut<'a, R> {
    type Creator = MutableResourceCreator<R>;
}

pub struct MutableResourceCreator<R: Resource>(PhantomData<R>);

impl<'a, R: Resource> ResourceCreator<'a> for MutableResourceCreator<R> {
    type Item = RefMut<'a, R>;

    fn create(resources: &'a Resources) -> Option<Self::Item> {
        resources.get_mut::<R>().ok()
    }
}

macro_rules! impl_query_tuple {
    (($($R:ident),*)) => {
        impl<'a, $($R: ResourceQuery, )*> ResourceCreator<'a> for ($($R,)*) {
            type Item = (
                $(<<$R as ResourceQuery>::Creator as ResourceCreator<'a>>::Item,)*
            );

            #[allow(unused_variables)]
            fn create(resources: &'a Resources) -> Option<Self::Item> {
                Some((
                    $($R::Creator::create(resources)?,)*
                ))
            }
        }

        impl<'a, $($R: ResourceQuery, )*> ResourceQuery for ($($R,)*) {
            type Creator = ($($R,)*);
        }
    };
}

// This might be extended (or maybe even make a macro for that
impl_query_tuple!(());
impl_query_tuple!((Ra));
impl_query_tuple!((Ra, Rb));
impl_query_tuple!((Ra, Rb, Rc));
impl_query_tuple!((Ra, Rb, Rc, Rd));
impl_query_tuple!((Ra, Rb, Rc, Rd, Re));
