//! Utility crate that contains useful, common functionality.
#![warn(missing_docs)]

pub mod collision;
pub mod color;
pub mod container;
mod loader;
pub mod math;
pub mod pathing;
mod rectangle;

pub use loader::{FileLoader, FileNotFoundError};
pub use rectangle::Rectangle;
