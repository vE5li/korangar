#![feature(decl_macro)]
#![feature(inline_const)]
#![feature(let_chains)]
#![feature(thread_local)]

#[macro_use]
pub mod logging;
#[macro_use]
pub mod profiling;

pub use debug_procedural::{debug_condition, profile};
