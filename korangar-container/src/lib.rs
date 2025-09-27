//! Implements some useful containers.
#![warn(missing_docs)]
#![cfg_attr(feature = "interface", feature(negative_impls))]
#![cfg_attr(feature = "interface", feature(impl_trait_in_assoc_type))]

use std::fmt::Formatter;

/// Easily creates typed key keys for a simple slab.
#[macro_export]
macro_rules! create_simple_key {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash)]
        pub struct $name(u32);

        impl $crate::SimpleKey for $name {
            fn new(key: u32) -> Self {
                Self(key)
            }

            fn key(&self) -> u32 {
                self.0
            }
        }
    };
    ($name:ident) => {
        create_simple_key!($name, "no documentation");
    };
}

/// Easily creates typed key keys for a generational slab.
#[macro_export]
macro_rules! create_generational_key {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash)]
        pub struct $name {
            key: u32,
            generation: core::num::NonZeroU32,
        }

        impl $crate::GenerationalKey for $name {
            fn new(key: u32, generation: core::num::NonZeroU32) -> Self {
                Self { key, generation }
            }

            fn key(&self) -> u32 {
                self.key
            }

            fn generation(&self) -> core::num::NonZeroU32 {
                self.generation
            }
        }
    };
    ($name:ident) => {
        create_generational_key!($name, "no documentation");
    };
}

mod generational_slab;
mod simple_cache;
mod simple_slab;

pub use generational_slab::{GenerationalIter, GenerationalKey, GenerationalSlab, SecondaryGenerationalSlab};
pub use simple_cache::{CacheError, CacheStatistics, Cacheable, SimpleCache};
pub use simple_slab::{SecondarySimpleSlab, SimpleIterator, SimpleKey, SimpleSlab};

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
