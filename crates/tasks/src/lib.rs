use std::ops::Deref;

// Some commonly used types to be exported
pub use async_channel as channel;
pub use async_lock as lock;
pub use crossbeam_channel as sync_channel;
pub use futures_lite as futures;
pub use parking_lot as sync_lock;

pub mod task;
pub mod task_pool;
pub mod utilities;

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
