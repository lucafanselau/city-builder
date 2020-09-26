use uuid;

use std::marker::PhantomData;

#[derive(Debug, Clone)]
pub struct HandleId(pub uuid::Uuid);

impl HandleId {
    pub fn new() -> Self {
        HandleId(uuid::Uuid::new_v4())
    }
}

pub struct Handle<T> where T: 'static {
    id: HandleId,
    data: PhantomData<T>,
}

impl<T> Handle<T> {
    pub fn new() -> Self {
        Handle {
            id: HandleId::new(),
            data: PhantomData,
        }
    }
}

pub type BufferHandle = HandleId;