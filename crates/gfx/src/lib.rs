#![feature(is_sorted)]
#![feature(trait_alias)]
#![feature(associated_type_bounds)]

pub mod command;
pub(crate) mod compat;
pub mod context;
pub mod context_builder;
pub mod heapy;
mod memory_page;
pub(crate) mod plumber;
pub(crate) mod pool;
// pub(crate) mod swapper;

mod graph;
