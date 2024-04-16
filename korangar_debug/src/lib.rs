#![feature(thread_local)]
#![feature(lazy_cell)]
#![feature(decl_macro)]
#![feature(let_chains)]

#[macro_use]
mod logging;
#[macro_use]
mod profiling;

pub use debug_procedural::{debug_condition, profile};

pub use self::logging::*;
pub use self::profiling::*;
