use std::collections::hash_map::DefaultHasher;

// pub mod profiler;
use lazy_static::lazy_static;

// Global Hasher
lazy_static! {
    pub static ref HASHER: DefaultHasher = DefaultHasher::new();
}
