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
    (()) => {};
}

// Impl for tuple type, should be basis for macro_rules thingy (because we want tuples of arbitrary size)
impl<'a, Ra: ResourceQuery, Rb: ResourceQuery> ResourceCreator<'a> for (Ra, Rb) {
    type Item = (
        <<Ra as ResourceQuery>::Creator as ResourceCreator<'a>>::Item,
        <<Rb as ResourceQuery>::Creator as ResourceCreator<'a>>::Item,
    );

    fn create(resources: &'a Resources) -> Option<Self::Item> {
        Some((
            Ra::Creator::create(resources)?,
            Rb::Creator::create(resources)?,
        ))
    }
}

impl<'a, Ra: ResourceQuery, Rb: ResourceQuery> ResourceQuery for (Ra, Rb) {
    type Creator = (Ra, Rb);
}
