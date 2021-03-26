use std::collections::hash_map::DefaultHasher;

// pub mod profiler;
use lazy_static::lazy_static;

// Reexport anyhow and thiserror
pub use anyhow;
pub use thiserror;

// Global Hasher
lazy_static! {
    pub static ref HASHER: DefaultHasher = DefaultHasher::new();
}
