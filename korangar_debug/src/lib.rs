#![feature(thread_local)]
#![feature(lazy_cell)]
#![feature(decl_macro)]
#![feature(let_chains)]

#[macro_use]
mod logging;
#[macro_use]
mod profiling;

pub use self::logging::*;
pub use self::profiling::*;
