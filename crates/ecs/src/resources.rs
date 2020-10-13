use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::error::Error;

use std::cell::{Ref, RefCell, RefMut};
use thiserror::Error;

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

impl Resources {
    /// Constructs an empty Resources Object
    pub fn new() -> Self {
        Resources {
            storage: HashMap::default(),
        }
    }

    pub fn insert<T: 'static>(&mut self, initial: T) -> Result<(), InsertResourceError> {
        let type_id = TypeId::of::<T>();
        if self.storage.contains_key(&type_id) {
            Err(InsertResourceError::DuplicateResource(type_id))
        } else {
            self.storage
                .insert(type_id, Box::new(RefCell::new(initial)));
            Ok(())
        }
    }

    pub fn get<T: 'static>(&self) -> Result<Ref<T>, GetResourceError> {
        let type_id = TypeId::of::<T>();

        let store = self
            .storage
            .get(&type_id)
            .ok_or(GetResourceError::MissingResource(type_id))?
            .downcast_ref::<RefCell<T>>()
            .ok_or(GetResourceError::DowncastFailed)?;
        Ok(store.borrow())
    }

    pub fn get_mut<T: 'static>(&self) -> Result<RefMut<T>, GetResourceError> {
        let type_id = TypeId::of::<T>();
        let store = self
            .storage
            .get(&type_id)
            .ok_or(GetResourceError::MissingResource(type_id))?
            .downcast_ref::<RefCell<T>>()
            .ok_or(GetResourceError::DowncastFailed)?;
        Ok(store.borrow_mut())
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
        resource.insert(my_resource);
        let my_second_resource = MySecondResource {
            also_a_fucking_counter: SECOND_INITIAL_COUNTER,
        };
        resource.insert(my_second_resource);
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
        let mut resource = test_setup();
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
        let mut one = resources.get_mut::<MyResource>().unwrap();
        // now this should panic
        let mut two = resources.get_mut::<MyResource>().unwrap();
    }
}
