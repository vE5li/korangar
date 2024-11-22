//! Utility crate that contains useful, common functionality.
#![warn(missing_docs)]
#![feature(let_chains)]

pub mod collision;
pub mod container;
mod loader;
pub mod math;
mod rectangle;
pub mod texture_atlas;

pub use loader::{FileLoader, FileNotFoundError};
pub use rectangle::Rectangle;
