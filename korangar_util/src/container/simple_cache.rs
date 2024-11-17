use std::borrow::Borrow;
use std::hash::Hash;
use std::num::{NonZeroU32, NonZeroUsize};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::Arc;

use hashbrown::HashMap;

use crate::container::{Cacheable, GenerationalSlab, Lru, Statistics, ValueTooBig};

create_generational_key!(SimpleCacheKey, "A key for a simple key");

/// A cache that holds a certain amount of values, limited by count and size.
/// Designed to be the owner of the cached values.
pub struct SimpleCache<K, V> {
    statistics: Arc<Statistics>,
    lookup: HashMap<K, SimpleCacheKey>,
    values: GenerationalSlab<SimpleCacheKey, V>,
    cache: Lru<SimpleCacheKey, K>,
}

impl<K: Clone + Eq + Hash, V: Cacheable> SimpleCache<K, V> {
    /// Creates a new cache that holds at most `max_count` values that are at
    /// most `max_size` bytes in size.
    pub fn new(max_count: NonZeroU32, max_size: NonZeroUsize) -> Self {
        let lookup = HashMap::new();
        let values = GenerationalSlab::with_capacity(max_count.get());

        let cache_capacity = max_count;
        let cache = Lru::new(cache_capacity);

        let statistics = Arc::new(Statistics {
            count: AtomicU32::new(0),
            max_count,
            size: AtomicUsize::new(0),
            max_size,
        });

        Self {
            statistics,
            lookup,
            values,
            cache,
        }
    }

    /// Returns the statistics of the cache.
    #[inline(always)]
    pub fn statistics(&self) -> Arc<Statistics> {
        self.statistics.clone()
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

    /// Inserts a value of the given size. The cache saves the given value and
    /// will drop it when there is not enough size for new cache entries.
    ///
    /// If the cache value is too big to be saved inside the cache, this
    /// function will return a [`ValueTooBig`] error.
    pub fn insert(&mut self, key: K, value: V) -> Result<(), ValueTooBig> {
        let size = value.size();

        if size > self.max_size() {
            return Err(ValueTooBig);
        }

        let _ = self.remove(&key);

        // Drop as many values as we need to fit the new value.
        while self.cache.count() > self.statistics.max_count.get().saturating_sub(1)
            || self.cache.size() > self.statistics.max_size.get().saturating_sub(size)
        {
            let (_, key) = self.cache.pop().ok_or(ValueTooBig)?;
            let cache_key = self.lookup.remove(&key).unwrap();
            let _ = self.values.remove(cache_key);
        }

        let cache_key = self.values.insert(value).expect("slab is full");
        self.lookup.insert(key.clone(), cache_key);
        self.cache.put(cache_key, key, size);
        self.update_statistics();

        Ok(())
    }

    /// Returns a reference of the given cached value.
    #[must_use]
    pub fn get<Q>(&mut self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.lookup.get(key).and_then(|&cache_key| {
            self.cache.touch(cache_key);
            self.values.get(cache_key)
        })
    }

    /// Removes the value with the given key from the cache.
    #[must_use]
    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.lookup.remove(key).and_then(|cache_key| {
            self.cache.remove(cache_key);
            let value = self.values.remove(cache_key);
            self.update_statistics();
            value
        })
    }

    fn update_statistics(&self) {
        self.statistics.count.store(self.cache.count(), Ordering::Release);
        self.statistics.size.store(self.cache.size(), Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use std::num::{NonZeroU32, NonZeroUsize};

    use super::*;

    #[test]
    fn test_new_cache() {
        let cache: SimpleCache<String, Vec<u8>> = SimpleCache::new(NonZeroU32::new(10).unwrap(), NonZeroUsize::new(1000).unwrap());
        assert_eq!(cache.max_count(), 10);
        assert_eq!(cache.max_size(), 1000);
        assert_eq!(cache.size(), 0);
    }

    #[test]
    fn test_insert_and_get() {
        let mut cache = SimpleCache::new(NonZeroU32::new(2).unwrap(), NonZeroUsize::new(100).unwrap());

        let value = vec![1, 2, 3];
        cache.insert("key1".to_string(), value.clone()).unwrap();

        assert_eq!(cache.get("key1"), Some(&value));
        assert_eq!(cache.get("nonexistent"), None);
    }

    #[test]
    fn test_remove() {
        let mut cache = SimpleCache::new(NonZeroU32::new(2).unwrap(), NonZeroUsize::new(100).unwrap());

        let value = vec![1, 2, 3];
        cache.insert("key1".to_string(), value.clone()).unwrap();

        assert_eq!(cache.remove(&"key1".to_string()), Some(value.clone()));
        assert_eq!(cache.get("key1"), None);
        assert_eq!(cache.remove(&"key1".to_string()), None);
    }

    #[test]
    fn test_size_limit() {
        let mut cache = SimpleCache::new(NonZeroU32::new(5).unwrap(), NonZeroUsize::new(10).unwrap());

        assert!(cache.insert("small".to_string(), vec![1, 2, 3]).is_ok());

        let result = cache.insert("big".to_string(), vec![1; 11]);
        assert!(matches!(result, Err(ValueTooBig)));
    }

    #[test]
    fn test_count_limit() {
        let mut cache = SimpleCache::new(NonZeroU32::new(2).unwrap(), NonZeroUsize::new(100).unwrap());

        cache.insert("key1".to_string(), vec![1]).unwrap();
        cache.insert("key2".to_string(), vec![2]).unwrap();

        cache.insert("key3".to_string(), vec![3]).unwrap();

        assert_eq!(cache.get("key1"), None);
        assert!(cache.get("key2").is_some());
        assert!(cache.get("key3").is_some());
    }

    #[test]
    fn test_statistics_initial_state() {
        let cache: SimpleCache<String, Vec<u8>> = SimpleCache::new(NonZeroU32::new(10).unwrap(), NonZeroUsize::new(1000).unwrap());

        let snapshot = cache.statistics().snapshot();
        assert_eq!(snapshot.count, 0);
        assert_eq!(snapshot.max_count, 10);
        assert_eq!(snapshot.size, 0);
        assert_eq!(snapshot.max_size, 1000);
    }

    #[test]
    fn test_statistics_after_insert() {
        let mut cache = SimpleCache::new(NonZeroU32::new(5).unwrap(), NonZeroUsize::new(100).unwrap());

        cache.insert("key1".to_string(), vec![1, 2, 3]).unwrap();

        let snapshot = cache.statistics().snapshot();
        assert_eq!(snapshot.count, 1);
        assert_eq!(snapshot.size, 3);

        cache.insert("key2".to_string(), vec![4, 5, 6, 7]).unwrap();

        let new_snapshot = cache.statistics().snapshot();
        assert_eq!(new_snapshot.count, 2);
        assert_eq!(new_snapshot.size, 7);
    }

    #[test]
    fn test_statistics_after_remove() {
        let mut cache = SimpleCache::new(NonZeroU32::new(5).unwrap(), NonZeroUsize::new(100).unwrap());

        cache.insert("key1".to_string(), vec![1, 2, 3]).unwrap();
        cache.insert("key2".to_string(), vec![4, 5, 6]).unwrap();

        let snapshot = cache.statistics().snapshot();
        assert_eq!(snapshot.count, 2);
        assert_eq!(snapshot.size, 6);

        let _ = cache.remove("key1");

        let new_snapshot = cache.statistics().snapshot();
        assert_eq!(new_snapshot.count, 1);
        assert_eq!(new_snapshot.size, 3);
    }

    #[test]
    fn test_statistics_after_eviction() {
        let mut cache = SimpleCache::new(NonZeroU32::new(2).unwrap(), NonZeroUsize::new(100).unwrap());

        cache.insert("key1".to_string(), vec![1, 2]).unwrap();
        cache.insert("key2".to_string(), vec![3, 4]).unwrap();

        let snapshot = cache.statistics().snapshot();
        assert_eq!(snapshot.count, 2);
        assert_eq!(snapshot.size, 4);

        cache.insert("key3".to_string(), vec![5, 6, 5]).unwrap();

        let new_snapshot = cache.statistics().snapshot();
        assert_eq!(new_snapshot.count, 2);
        assert_eq!(new_snapshot.size, 5);
    }
}
