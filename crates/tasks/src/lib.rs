use std::ops::Deref;

pub use futures_lite as futures;

pub mod task;
pub mod task_pool;

/// Compute pool (intended to use in )
pub struct ComputePool(task_pool::TaskPool);

impl Deref for ComputePool {
    type Target = task_pool::TaskPool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ComputePool {
    pub fn new() -> Self {
        let cpus = (num_cpus::get() as f32 / 2.0).ceil();
        Self(task_pool::TaskPool::new(Some(cpus as _), "compute_pool"))
    }
}

impl Default for ComputePool {
    fn default() -> Self {
        Self::new()
    }
}

/// Asynchronous Compute Pool
pub struct AsyncComputePool(task_pool::TaskPool);

impl Deref for AsyncComputePool {
    type Target = task_pool::TaskPool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsyncComputePool {
    pub fn new() -> Self {
        let cpus = (num_cpus::get() as f32 / 2.0).floor();
        Self(task_pool::TaskPool::new(
            Some(cpus as _),
            "async_compute_pool",
        ))
    }
}

impl Default for AsyncComputePool {
    fn default() -> Self {
        Self::new()
    }
}
