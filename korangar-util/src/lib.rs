//! Utility crate that contains useful, common functionality.
#![warn(missing_docs)]
#![cfg_attr(feature = "interface", feature(negative_impls))]
#![cfg_attr(feature = "interface", feature(impl_trait_in_assoc_type))]

pub mod collision;
pub mod color;
pub mod container;
mod loader;
pub mod math;
pub mod pathing;
mod rectangle;

pub use loader::{FileLoader, FileNotFoundError};
pub use rectangle::Rectangle;
