//! Utility crate that contains useful, common functionality.
#![warn(missing_docs)]
#![feature(let_chains)]

pub mod collision;
pub mod container;
mod loader;
pub mod math;

pub use loader::{FileLoader, FileNotFoundError};
