//! Implements some useful containers.

/// Easily creates typed key keys for a simple slab.
#[macro_export]
macro_rules! create_simple_key {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash)]
        pub struct $name(u32);

        impl $crate::container::SimpleKey for $name {
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

        impl $crate::container::GenerationalKey for $name {
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
