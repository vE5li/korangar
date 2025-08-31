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

use std::fmt::Formatter;

pub use loader::{FileLoader, FileNotFoundError};
pub use rectangle::Rectangle;

/// Bytes that are displayed with SI units.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct HumanReadableBytes(usize);

impl std::fmt::Display for HumanReadableBytes {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB", "PiB"];
        const THRESHOLD: f64 = 1024.0;

        if self.0 == 0 {
            return "0 B".fmt(f);
        }

        let bytes_f = self.0 as f64;
        let mut unit_index = 0;
        let mut size = bytes_f;

        while size >= THRESHOLD && unit_index < UNITS.len() - 1 {
            size /= THRESHOLD;
            unit_index += 1;
        }

        if unit_index == 0 {
            write!(f, "{} {}", self.0, UNITS[unit_index])
        } else if size >= 100.0 {
            write!(f, "{:.0} {}", size, UNITS[unit_index])
        } else if size >= 10.0 {
            write!(f, "{:.1} {}", size, UNITS[unit_index])
        } else {
            write!(f, "{:.2} {}", size, UNITS[unit_index])
        }
    }
}
