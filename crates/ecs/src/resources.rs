use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::error::Error;

use thiserror::Error;

// Basically a Map that provides a centralised storage for Resources
pub struct Resources {
    // We probably find, that this type of storage is not sufficient
    storage: HashMap<TypeId, Box<dyn Any>>,
}

#[derive(Debug, Error)]
pub enum GetResourceError {
    #[error("There is no Ressource for TypeId '{0:?}'")]
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
            self.storage.insert(type_id, Box::new(initial));
            Ok(())
        }
    }

    pub fn get<T: 'static>(&self) -> Result<&T, GetResourceError> {
        let type_id = TypeId::of::<T>();
        let entry = self
            .storage
            .get(&type_id)
            .ok_or(GetResourceError::MissingResource(type_id))?;
        entry
            .downcast_ref::<T>()
            .ok_or(GetResourceError::DowncastFailed)
    }

    pub fn get_mut<T: 'static>(&mut self) -> Result<&mut T, GetResourceError> {
        let type_id = TypeId::of::<T>();
        let entry = self
            .storage
            .get_mut(&type_id)
            .ok_or(GetResourceError::MissingResource(type_id))?;
        entry
            .downcast_mut::<T>()
            .ok_or(GetResourceError::DowncastFailed)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    struct MyResource {
        counter: u32,
    }

    fn test_setup() -> Resources {
        let mut resource = Resources::new();

        let my_resource = MyResource { counter: 537 };
        resource.insert(my_resource);

        resource
    }

    #[test]
    fn first_resource() {
        let resource = test_setup();
        // and now we will try to retrieve this value
        let my_ref = resource
            .get::<MyResource>()
            .expect("failed to get the resource");
        assert_eq!(my_ref.counter, 537);
    }

    #[test]
    fn mutable_resource() {
        let mut resource = test_setup();
        // and modify that
        {
            let my_mut_ref = resource
                .get_mut::<MyResource>()
                .expect("failed to get that resource");
            my_mut_ref.counter += 1;
        }

        // and now assert that
        assert_eq!(resource.get::<MyResource>().unwrap().counter, 538);
    }
}
