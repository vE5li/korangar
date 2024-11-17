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
mod lru;
mod simple_cache;
mod simple_slab;

use std::num::{NonZeroU32, NonZeroUsize};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::Arc;

pub use generational_slab::{GenerationalIter, GenerationalKey, GenerationalSlab, SecondaryGenerationalSlab};
pub(crate) use lru::Lru;
pub use simple_cache::SimpleCache;
pub use simple_slab::{SecondarySimpleSlab, SimpleIterator, SimpleKey, SimpleSlab};

/// Something that can be cached.
pub trait Cacheable {
    /// Must return the size of the object. The size can be the actual byte size
    /// of a struct or the size that is allocated for an external resource.
    fn size(&self) -> usize;
}

impl Cacheable for Vec<u8> {
    fn size(&self) -> usize {
        self.len()
    }
}

impl<T: Cacheable> Cacheable for Arc<T> {
    fn size(&self) -> usize {
        self.as_ref().size()
    }
}

/// Thrown when a value is too big for the cache to store.
#[derive(Debug)]
pub struct ValueTooBig;

impl std::fmt::Display for ValueTooBig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Value is too big")
    }
}

impl std::error::Error for ValueTooBig {}

/// Statistic about the cache.
#[derive(Debug)]
pub struct Statistics {
    count: AtomicU32,
    max_count: NonZeroU32,
    size: AtomicUsize,
    max_size: NonZeroUsize,
}

/// Returns a snapshot view of the statistics.
#[derive(Debug)]
pub struct Snapshot {
    /// The current count of values inside the cache.
    pub count: u32,
    /// The maximal count of values inside the cache.
    pub max_count: u32,
    /// The current size of values inside the cache.
    pub size: usize,
    /// The maximal size of values inside the cache.
    pub max_size: usize,
}

impl Statistics {
    /// Returns a snapshot of the current values of the cache statistics.
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            count: self.count.load(Ordering::Acquire),
            max_count: self.max_count.get(),
            size: self.size.load(Ordering::Acquire),
            max_size: self.max_size.get(),
        }
    }
}
