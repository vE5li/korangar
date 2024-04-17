#![feature(thread_local)]
#![feature(lazy_cell)]
#![feature(decl_macro)]
#![feature(let_chains)]

#[macro_use]
mod logging;
#[macro_use]
mod profiling;

pub use debug_procedural::{debug_condition, profile};

pub use self::logging::{print_debug, print_indented, Colorize, Colorized, Timer};
pub use self::profiling::*;
