use std::marker::PhantomData;
use std::num::{NonZeroU32, NonZeroUsize};
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

use super::{Cacheable, GenerationalKey, Lru, SimpleIterator, SimpleKey, SimpleSlab, Statistics, ValueTooBig};

/// A cache that holds a certain amount of values, limited by count and size.
/// Designed to be used for external resources (for example GPU objects).
pub struct ResourceCache<I, V> {
    statistics: Arc<Statistics>,
    values: SimpleSlab<u32, V>,
    cache: Lru<I, u32>,
    _marker: PhantomData<I>,
}

impl<I: GenerationalKey + Copy, V: Cacheable> ResourceCache<I, V> {
    /// Creates a new cache that holds at most `max_count` values that are at
    /// most `max_size` bytes in size.
    pub fn new(max_count: NonZeroU32, max_size: NonZeroUsize) -> Self {
        let values = SimpleSlab::with_capacity(max_count.get());

        let cache_capacity = max_count;
        let cache = Lru::new(cache_capacity);

        let statistics = Arc::new(Statistics {
            count: AtomicU32::new(0),
            max_count,
            size: AtomicUsize::new(0),
            max_size,
            version: AtomicU64::new(0),
        });

        Self {
            statistics,
            values,
            cache,
            _marker: PhantomData,
        }
    }

    /// Returns the statistics of the cache.
    #[inline(always)]
    pub fn statistics(&self) -> Arc<Statistics> {
        self.statistics.clone()
    }

    /// Returns the current version of the texture set. Each mutation increases
    /// the version by one.
    #[inline(always)]
    pub fn version(&self) -> u64 {
        self.statistics.version.load(Ordering::Acquire)
    }

    /// Returns the current count of values inside the cache.
    #[inline(always)]
    pub fn count(&self) -> u32 {
        self.statistics.count.load(Ordering::Acquire)
    }

    /// Returns the maximal count of values inside the cache.
    #[inline(always)]
    pub fn max_count(&self) -> u32 {
        self.statistics.max_count.get()
    }

    /// Returns the current size of all values inside cache.
    #[inline(always)]
    pub fn size(&self) -> usize {
        self.statistics.size.load(Ordering::Acquire)
    }

    /// Returns the maximal count of values inside the cache.
    #[inline(always)]
    pub fn max_size(&self) -> usize {
        self.statistics.max_size.get()
    }

    /// Clears the cache.
    pub fn clear(&mut self) {
        self.cache = Lru::new(self.statistics.max_count);

        self.values.clear();
        self.statistics.count.store(0, Ordering::Release);
        self.statistics.size.store(0, Ordering::Release);
        let _ = self
            .statistics
            .version
            .fetch_update(Ordering::Release, Ordering::Acquire, |v| Some(v + 1));
    }

    /// Inserts a value of the given size. The cache saves the given value and
    /// will drop it when there is not enough size for new cache entries, but
    /// the key to find the cached entry must be created by using an external
    /// slab. This way the value of a cache entry resides inside the cache, but
    /// all information to re-load the cached value is tracked outside.
    ///
    /// This makes it possible to cache texture data on the GPU inside this
    /// cache, but dynamically re-load old texture again, if they are used
    /// again.
    ///
    /// If the cache value is too big to be saved inside the cache, this
    /// function will return a [`ValueTooBig`] error.
    pub fn insert(&mut self, key: I, value: V) -> Result<(), ValueTooBig> {
        let size = value.size();

        if size > self.max_size() {
            return Err(ValueTooBig);
        }

        if self.cache.contains_key(key) {
            let _ = self.remove(key);
        }

        // Drop as many values as we need to fit the new value.
        while self.cache.count() > self.statistics.max_count.get().saturating_sub(1)
            || self.cache.size() > self.statistics.max_size.get().saturating_sub(size)
        {
            let (_, cache_key) = self.cache.pop().ok_or(ValueTooBig)?;
            let _ = self.values.remove(cache_key);
        }

        let fixed_key = self.values.insert(value).expect("slab is full");
        self.cache.put(key, fixed_key, size);
        self.update_statistics();

        Ok(())
    }

    /// Marks the value as recently used.
    #[inline(always)]
    pub fn touch(&mut self, key: I) {
        self.cache.touch(key)
    }

    /// Returns the cache entry index of the given value if it is still cached.
    /// Useful if you need the position of a specific value inside the
    /// [`ResourceCache::values()`] iterator.
    ///
    /// Very usefully for creating the offsets for nun uniform indexing of
    /// textures.
    #[inline(always)]
    #[must_use]
    pub fn fixed_index(&mut self, key: I) -> Option<u32> {
        self.cache.get(key).map(|key| key.key())
    }

    /// Returns a reference of the given cached value.
    #[inline(always)]
    #[must_use]
    pub fn get(&mut self, key: I) -> Option<&V> {
        let fixed_key = *self.cache.get(key)?;
        self.values.get(fixed_key)
    }

    /// Returns a mutable reference of the given cached value.
    #[inline(always)]
    #[must_use]
    pub fn get_mut(&mut self, key: I) -> Option<&V> {
        let fixed_key = *self.cache.get(key)?;
        self.values.get(fixed_key)
    }

    /// Tests if the value with the given key is still cached.
    pub fn contains(&self, key: I) -> bool {
        self.cache.contains_key(key)
    }

    /// Returns an iterator over all non-empty cache entry slots.
    #[inline(always)]
    #[must_use]
    pub fn values(&self) -> SimpleIterator<u32, V> {
        self.values.iter()
    }

    /// Removes the value with the given key from the set.
    #[must_use]
    pub fn remove(&mut self, key: I) -> Option<V> {
        self.cache.remove(key).and_then(|fixed_key| {
            let value = self.values.remove(fixed_key);
            self.update_statistics();
            value
        })
    }

    fn update_statistics(&self) {
        self.statistics.count.store(self.cache.count(), Ordering::Release);
        self.statistics.size.store(self.cache.size(), Ordering::Release);
        let _ = self
            .statistics
            .version
            .fetch_update(Ordering::Release, Ordering::Acquire, |v| Some(v + 1));
    }
}

#[cfg(test)]
mod tests {
    use std::num::{NonZeroU32, NonZeroUsize};

    use crate::container::{GenerationalKey, ResourceCache};

    create_generational_key!(TestKey);

    #[test]
    fn test_resource_cache_creation() {
        let cache: ResourceCache<TestKey, Vec<u8>> = ResourceCache::new(NonZeroU32::new(10).unwrap(), NonZeroUsize::new(1000).unwrap());
        assert_eq!(cache.count(), 0);
        assert_eq!(cache.size(), 0);
        assert_eq!(cache.max_count(), 10);
        assert_eq!(cache.max_size(), 1000);
    }

    #[test]
    fn test_resource_cache_insert() {
        let mut cache: ResourceCache<TestKey, Vec<u8>> = ResourceCache::new(NonZeroU32::new(10).unwrap(), NonZeroUsize::new(1000).unwrap());
        let result = cache.insert(TestKey::new(0, NonZeroU32::new(1).unwrap()), vec![0u8; 5]);
        assert!(result.is_ok());
        assert_eq!(cache.count(), 1);
        assert_eq!(cache.size(), 5);
    }

    #[test]
    fn test_resource_cache_get() {
        let mut cache: ResourceCache<TestKey, Vec<u8>> = ResourceCache::new(NonZeroU32::new(10).unwrap(), NonZeroUsize::new(1000).unwrap());
        cache.insert(TestKey::new(0, NonZeroU32::new(1).unwrap()), vec![2u8; 1]).unwrap();
        let value = cache.get(TestKey::new(0, NonZeroU32::new(1).unwrap()));
        assert_eq!(value, Some(&vec![2u8; 1]));
    }

    #[test]
    fn test_resource_cache_remove() {
        let mut cache: ResourceCache<TestKey, Vec<u8>> = ResourceCache::new(NonZeroU32::new(10).unwrap(), NonZeroUsize::new(1000).unwrap());
        cache.insert(TestKey::new(0, NonZeroU32::new(1).unwrap()), vec![0u8; 16]).unwrap();
        let removed = cache.remove(TestKey::new(0, NonZeroU32::new(1).unwrap()));
        assert_eq!(removed, Some(vec![0u8; 16]));
        assert_eq!(cache.count(), 0);
    }

    #[test]
    fn test_resource_cache_clear() {
        let mut cache: ResourceCache<TestKey, Vec<u8>> = ResourceCache::new(NonZeroU32::new(10).unwrap(), NonZeroUsize::new(1000).unwrap());
        cache.insert(TestKey::new(0, NonZeroU32::new(1).unwrap()), vec![1u8; 1]).unwrap();
        cache.insert(TestKey::new(1, NonZeroU32::new(1).unwrap()), vec![2u8; 1]).unwrap();
        cache.clear();
        assert_eq!(cache.count(), 0);
        assert_eq!(cache.size(), 0);
    }

    #[test]
    fn test_resource_cache_overflow() {
        let mut cache: ResourceCache<TestKey, Vec<u8>> = ResourceCache::new(NonZeroU32::new(2).unwrap(), NonZeroUsize::new(200).unwrap());
        cache.insert(TestKey::new(0, NonZeroU32::new(1).unwrap()), vec![2u8; 1]).unwrap();
        cache.insert(TestKey::new(1, NonZeroU32::new(1).unwrap()), vec![3u8; 1]).unwrap();
        let result = cache.insert(TestKey::new(2, NonZeroU32::new(1).unwrap()), vec![4u8; 1]);
        assert!(result.is_ok());
        assert_eq!(cache.count(), 2);
    }

    #[test]
    fn test_value_too_big() {
        let mut cache: ResourceCache<TestKey, Vec<u8>> = ResourceCache::new(NonZeroU32::new(10).unwrap(), NonZeroUsize::new(8).unwrap());
        let result = cache.insert(TestKey::new(0, NonZeroU32::new(1).unwrap()), vec![0u8; 16]);
        assert!(result.is_err());
    }

    #[test]
    fn test_resource_cache_version() {
        let mut cache: ResourceCache<TestKey, Vec<u8>> = ResourceCache::new(NonZeroU32::new(10).unwrap(), NonZeroUsize::new(1000).unwrap());
        let initial_version = cache.version();
        cache.insert(TestKey::new(0, NonZeroU32::new(1).unwrap()), vec![0u8; 1]).unwrap();
        assert!(cache.version() > initial_version);
    }

    #[test]
    fn test_resource_cache_fixed_key() {
        let mut cache: ResourceCache<TestKey, Vec<u8>> = ResourceCache::new(NonZeroU32::new(10).unwrap(), NonZeroUsize::new(1000).unwrap());
        cache.insert(TestKey::new(0, NonZeroU32::new(1).unwrap()), vec![0u8; 1]).unwrap();
        let fixed_key = cache.fixed_index(TestKey::new(0, NonZeroU32::new(1).unwrap()));
        assert_eq!(fixed_key, Some(0));
    }

    #[test]
    fn test_resource_cache_lru() {
        let mut cache: ResourceCache<TestKey, Vec<u8>> = ResourceCache::new(NonZeroU32::new(3).unwrap(), NonZeroUsize::new(1000).unwrap());

        cache.insert(TestKey::new(0, NonZeroU32::new(1).unwrap()), vec![1u8; 1]).unwrap();
        cache.insert(TestKey::new(1, NonZeroU32::new(1).unwrap()), vec![2u8; 1]).unwrap();
        cache.insert(TestKey::new(2, NonZeroU32::new(1).unwrap()), vec![3u8; 1]).unwrap();

        assert_eq!(cache.count(), 3);

        cache.touch(TestKey::new(0, NonZeroU32::new(1).unwrap()));
        cache.insert(TestKey::new(3, NonZeroU32::new(1).unwrap()), vec![4u8; 1]).unwrap();
        assert_eq!(cache.count(), 3);
        assert!(cache.get(TestKey::new(1, NonZeroU32::new(1).unwrap())).is_none());
        assert_eq!(cache.get(TestKey::new(0, NonZeroU32::new(1).unwrap())), Some(&vec![1u8; 1]));
        assert_eq!(cache.get(TestKey::new(2, NonZeroU32::new(1).unwrap())), Some(&vec![3u8; 1]));
        assert_eq!(cache.get(TestKey::new(3, NonZeroU32::new(1).unwrap())), Some(&vec![4u8; 1]));

        cache.touch(TestKey::new(2, NonZeroU32::new(1).unwrap()));
        cache.insert(TestKey::new(4, NonZeroU32::new(1).unwrap()), vec![5u8; 1]).unwrap();
        assert_eq!(cache.count(), 3);
        assert!(cache.get(TestKey::new(0, NonZeroU32::new(1).unwrap())).is_none());
        assert_eq!(cache.get(TestKey::new(2, NonZeroU32::new(1).unwrap())), Some(&vec![3u8; 1]));
        assert_eq!(cache.get(TestKey::new(3, NonZeroU32::new(1).unwrap())), Some(&vec![4u8; 1]));
        assert_eq!(cache.get(TestKey::new(4, NonZeroU32::new(1).unwrap())), Some(&vec![5u8; 1]));
    }
}
