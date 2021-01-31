use std::any::{Any, TypeId};
use std::collections::HashMap;

use crate::resource::{ResourceCreator, ResourceQuery};
use std::cell::{Ref, RefCell, RefMut};
use thiserror::Error;

// Resource type
pub trait Resource: Any + 'static {}
impl<T: Any + 'static> Resource for T {}

// Basically a Map that provides a centralised storage for Resources
pub struct Resources {
    // We probably find, that this type of storage is not sufficient
    storage: HashMap<TypeId, Box<dyn Any>>,
}

#[derive(Debug, Error)]
pub enum GetResourceError {
    #[error("There is no Resource for TypeId '{0:?}'")]
    MissingResource(TypeId),
    #[error("Downcast failed")]
    DowncastFailed,
}

#[derive(Debug, Error)]
pub enum InsertResourceError {
    #[error("Resource needs to have a unique type '{0:?}'")]
    DuplicateResource(TypeId),
}

impl Default for Resources {
    fn default() -> Self {
        Self::new()
    }
}

impl Resources {
    /// Constructs an empty Resources Object
    pub fn new() -> Self {
        Resources {
            storage: HashMap::default(),
        }
    }

    pub fn insert<T: Resource>(&mut self, initial: T) -> Result<(), InsertResourceError> {
        use std::collections::hash_map::Entry;
        let type_id = TypeId::of::<T>();
        match self.storage.entry(type_id) {
            Entry::Occupied(_) => Err(InsertResourceError::DuplicateResource(type_id)),
            Entry::Vacant(e) => {
                e.insert(Box::new(RefCell::new(initial)));
                Ok(())
            }
        }
    }

    pub fn get<T: Resource>(&self) -> Result<Ref<T>, GetResourceError> {
        let type_id = TypeId::of::<T>();
        let store = self
            .storage
            .get(&type_id)
            .ok_or(GetResourceError::MissingResource(type_id))?
            .downcast_ref::<RefCell<T>>()
            .ok_or(GetResourceError::DowncastFailed)?;
        Ok(store.borrow())
    }

    pub fn get_mut<T: Resource>(&self) -> Result<RefMut<T>, GetResourceError> {
        let type_id = TypeId::of::<T>();
        let store = self
            .storage
            .get(&type_id)
            .ok_or(GetResourceError::MissingResource(type_id))?
            .downcast_ref::<RefCell<T>>()
            .ok_or(GetResourceError::DowncastFailed)?;
        Ok(store.borrow_mut())
    }

    // TODO: Return type
    pub fn query<Q: ResourceQuery>(
        &self,
    ) -> Option<<<Q as ResourceQuery>::Creator as ResourceCreator>::Item> {
        <Q as ResourceQuery>::Creator::create(&self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MyResource {
        counter: u32,
    }

    struct MySecondResource {
        also_a_fucking_counter: u32,
    }

    const INITIAL_COUNTER: u32 = 537;
    const SECOND_INITIAL_COUNTER: u32 = 2;

    fn test_setup() -> Resources {
        let mut resource = Resources::new();
        let my_resource = MyResource {
            counter: INITIAL_COUNTER,
        };
        resource.insert(my_resource).expect("");
        let my_second_resource = MySecondResource {
            also_a_fucking_counter: SECOND_INITIAL_COUNTER,
        };
        resource.insert(my_second_resource).expect("");
        resource
    }

    #[test]
    fn first_resource() {
        let resource = test_setup();
        // and now we will try to retrieve this value
        let my_ref = resource
            .get::<MyResource>()
            .expect("failed to get the resource");
        assert_eq!(my_ref.counter, INITIAL_COUNTER);
    }

    #[test]
    fn mutable_resource() {
        let resource = test_setup();
        // and modify that
        {
            let mut my_mut_ref = resource
                .get_mut::<MyResource>()
                .expect("failed to get that resource");
            my_mut_ref.counter += 1;
        }

        // and now assert that
        assert_eq!(
            resource.get::<MyResource>().unwrap().counter,
            INITIAL_COUNTER + 1
        );
    }

    #[test]
    fn multiple_mut_resources() {
        // it should be possible to have 2 mut references to Resources at the same time
        let resources = test_setup();
        {
            let mut first = resources
                .get_mut::<MyResource>()
                .expect("failed to get mut first resource");
            let mut second = resources
                .get_mut::<MySecondResource>()
                .expect("failed to get mut second resource");
            // Now we have two active mutable references to the resources storage
            first.counter += 1;
            second.also_a_fucking_counter += 1;
        }
        // and now we check if that is correct
        assert_eq!(
            resources.get::<MyResource>().unwrap().counter,
            INITIAL_COUNTER + 1
        );
        assert_eq!(
            resources
                .get::<MySecondResource>()
                .unwrap()
                .also_a_fucking_counter,
            SECOND_INITIAL_COUNTER + 1
        );
    }

    #[test]
    #[should_panic]
    fn to_many_borrows() {
        let resources = test_setup();
        let _one = resources.get_mut::<MyResource>().unwrap();
        // now this should panic
        let _two = resources.get_mut::<MyResource>().unwrap();
    }

    #[test]
    fn resource_query() {
        let resources = test_setup();
        // The point is, that we should be able to have a resource_query that handles the ref creation
        {
            let mut resource = resources.query::<RefMut<MyResource>>().unwrap();
            resource.counter += 1;
        }
        let resource = resources.query::<Ref<MyResource>>().unwrap();
        assert_eq!(resource.counter, INITIAL_COUNTER + 1);
    }

    #[test]
    fn multiple_resource_query() {
        let resources = test_setup();
        // Now we are getting two resources at the same time
        {
            let (mut first, mut second) = resources
                .query::<(RefMut<MyResource>, RefMut<MySecondResource>)>()
                .unwrap();
            first.counter *= 2;
            second.also_a_fucking_counter *= 2;
        }
        // And now check that
        {
            let (first, second) = resources
                .query::<(Ref<MyResource>, Ref<MySecondResource>)>()
                .unwrap();
            assert_eq!(first.counter, INITIAL_COUNTER * 2);
            assert_eq!(second.also_a_fucking_counter, SECOND_INITIAL_COUNTER * 2);
        }
    }

    #[test]
    fn large_queries() {
        let resources = {
            let mut resources = Resources::new();
            resources.insert(1u32).unwrap();
            resources.insert(2i16).unwrap();
            resources.insert(true).unwrap();
            resources.insert("Hello world".to_string()).unwrap();
            resources
        };

        // Now to the large query
        let (unsigned, signed, boolean, string) = resources
            .query::<(Ref<u32>, Ref<i16>, Ref<bool>, Ref<String>)>()
            .unwrap();
        assert_eq!(*unsigned, 1);
        assert_eq!(*signed, 2);
        assert_eq!(*boolean, true);
        assert_eq!(string.as_str(), "Hello world");
    }

    struct MyGenericResource<T>(T);

    #[test]
    fn generic_resource() {
        let mut resources = Resources::new();

        resources.insert(MyGenericResource(37i32)).unwrap();
        resources.insert(MyGenericResource(1024i16)).unwrap();

        assert_eq!(resources.get::<MyGenericResource<i32>>().unwrap().0, 37);
        assert_eq!(resources.get::<MyGenericResource<i16>>().unwrap().0, 1024);
    }
}
