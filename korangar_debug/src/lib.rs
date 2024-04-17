#![feature(thread_local)]
#![feature(decl_macro)]
#![feature(let_chains)]

#[macro_use]
pub mod logging;
#[macro_use]
pub mod profiling;

pub use debug_procedural::{debug_condition, profile};
