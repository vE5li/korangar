use std::borrow::Borrow;
use std::collections::VecDeque;
use std::hash::{BuildHasher, Hash};
use std::num::{NonZeroU32, NonZeroUsize};
use std::sync::Arc;

use hashbrown::hash_map::RawEntryMut;
use hashbrown::{HashMap, HashSet};

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

/// Errors the cache can throw.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub enum CacheError {
    /// Thrown when a value is too big for the cache to store.
    ValueTooBig,
    /// Thrown when value is already present (double insert).
    ValueAlreadyPresent,
}

impl std::fmt::Display for CacheError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CacheError::ValueTooBig => {
                write!(f, "Value is too big")
            }
            CacheError::ValueAlreadyPresent => {
                write!(f, "Value is already present in cache")
            }
        }
    }
}

impl std::error::Error for CacheError {}

/// A FIFO-ordered ghost list that supports O(1) random access and removal.
/// Insertion have (because of evictions) mostly O(1), but has the worst case of
/// O(n) if all items of a queue are tombstones.
struct GhostList<K> {
    map: HashSet<K>,
    queue: VecDeque<K>,
    max_count: usize,
}

impl<K: Clone + Eq + Hash> GhostList<K> {
    fn new(max_count: usize) -> Self {
        Self {
            map: HashSet::new(),
            queue: VecDeque::new(),
            max_count,
        }
    }

    fn len(&self) -> usize {
        self.map.len()
    }

    fn contains<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.map.contains(key)
    }

    fn insert(&mut self, key: K) {
        if self.map.contains(&key) {
            return;
        }

        while self.len() >= self.max_count {
            self.evict_oldest();
        }

        self.map.insert(key.clone());
        self.queue.push_front(key);
    }

    fn remove<Q>(&mut self, key: &Q)
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        // We only remove the item from the lookup map. This means we create a tombstone
        // in the queue, so we need to occasionally remove the tombstones.
        self.map.remove(key);
    }

    fn evict_oldest(&mut self) -> Option<K> {
        while let Some(key) = self.queue.pop_back() {
            if self.map.contains(&key) {
                self.map.remove(&key);
                return Some(key);
            }
        }
        None
    }

    fn should_compact(&self) -> bool {
        self.queue.len() > self.map.len() * 2
    }

    /// Removes all tombstones while preserving the FIFO order. Check is
    /// compaction is really needed and will skip it not.
    fn compact(&mut self) {
        if self.should_compact() {
            let mut new_queue = VecDeque::with_capacity(self.map.len());

            // Rebuild queue with only live entries, preserving FIFO order.
            for key in self.queue.iter().rev() {
                if self.map.contains(key) {
                    new_queue.push_front(key.clone());
                }
            }

            self.queue = new_queue;
        }
    }
}

struct ValueEntry<V> {
    value: V,
    freq: u8,
    size: usize,
}

impl<V> ValueEntry<V> {
    fn new(value: V, size: usize) -> Self {
        Self { value, freq: 0, size }
    }
}

/// A cache that holds a certain amount of values, limited by count and size.
/// Designed to be the owner of the cached values and use the S3-FIFO cache
/// strategy.
pub struct SimpleCache<K, V> {
    values: HashMap<K, ValueEntry<V>>,

    small_fifo: VecDeque<K>,
    main_fifo: VecDeque<K>,
    ghost: GhostList<K>,

    small_count: u32,
    small_size: usize,
    max_small_count: u32,
    max_small_size: usize,
    main_count: u32,
    main_size: usize,
    max_main_count: u32,
    max_main_size: usize,
    max_count: u32,
    max_size: usize,
}

impl<K: Clone + Eq + Hash, V: Cacheable> SimpleCache<K, V> {
    /// Creates a new cache that holds at most `max_count` values that are at
    /// most `max_size` bytes in size.
    pub fn new(max_count: NonZeroU32, max_size: NonZeroUsize) -> Self {
        let max_count = max_count.get();
        let max_size = max_size.get();

        // Small FIFO gets 10% of capacity (minimum 1)
        let max_small_count = std::cmp::max(1, max_count / 10);
        let max_small_size = std::cmp::max(1, max_size / 10);

        Self {
            values: HashMap::new(),
            main_fifo: VecDeque::new(),
            small_fifo: VecDeque::new(),
            ghost: GhostList::new(max_count as usize - max_small_count as usize),
            small_count: 0,
            small_size: 0,
            max_small_count,
            max_small_size,
            main_count: 0,
            main_size: 0,
            max_main_count: max_count - max_small_count,
            max_main_size: max_size - max_small_size,
            max_count,
            max_size,
        }
    }

    /// Returns the current count of all values inside cache.
    #[inline(always)]
    pub fn count(&self) -> u32 {
        self.small_count + self.main_count
    }

    /// Returns the current size of all values inside cache.
    #[inline(always)]
    pub fn size(&self) -> usize {
        self.small_size + self.main_size
    }

    /// Returns the maximal count of values inside the cache.
    #[inline(always)]
    pub fn max_count(&self) -> u32 {
        self.max_count
    }

    /// Returns the maximal size of values inside the cache.
    #[inline(always)]
    pub fn max_size(&self) -> usize {
        self.max_size
    }

    /// Compacts the cache's ghost list, if there are many tombstones.
    pub fn compact(&mut self) {
        self.ghost.compact();
    }

    /// Inserts a value of the given size. The cache saves the given value and
    /// will drop it when there is not enough size for new cache entries.
    ///
    /// Returns an error if either the value it too big for the cache to store
    /// or if the item was already inserted.
    pub fn insert(&mut self, key: K, value: V) -> Result<(), CacheError> {
        let size = value.size();

        if self.values.contains_key(&key) {
            return Err(CacheError::ValueAlreadyPresent);
        }

        if size > self.max_small_size {
            return Err(CacheError::ValueTooBig);
        }

        if self.ghost.contains(&key) {
            self.ghost.remove(&key);

            while self.main_count >= self.max_main_count || self.main_size.saturating_add(size) > self.max_main_size {
                self.evict_m();
            }
            self.main_fifo.push_front(key.clone());
            self.main_count += 1;
            self.main_size += size;
        } else {
            while self.small_count >= self.max_small_count || self.small_size.saturating_add(size) > self.max_small_size {
                self.evict_s();
            }
            self.small_fifo.push_front(key.clone());
            self.small_count += 1;
            self.small_size += size;
        }

        self.values.insert(key, ValueEntry::new(value, size));

        Ok(())
    }

    /// Returns a reference of the given cached value. This is a special version
    /// of get that uses hashbrown's raw entry API to allow to query with
    /// borrowed data.
    #[must_use]
    pub fn get_with<Q, F>(&mut self, key: &Q, mut eq: F) -> Option<&V>
    where
        Q: Hash + ?Sized,
        F: FnMut(&K) -> bool,
    {
        let hash = self.values.hasher().hash_one(key);
        match self.values.raw_entry_mut().from_hash(hash, |k| eq(k)) {
            RawEntryMut::Vacant(_) => None,
            RawEntryMut::Occupied(entry) => {
                let value_entry = entry.into_mut();
                value_entry.freq = std::cmp::min(value_entry.freq + 1, 3);
                Some(&value_entry.value)
            }
        }
    }

    /// Returns a reference of the given cached value.
    #[must_use]
    pub fn get<Q>(&mut self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.values.get_mut(key).map(|value_entry| {
            value_entry.freq = std::cmp::min(value_entry.freq + 1, 3);
            &value_entry.value
        })
    }

    fn evict_s(&mut self) {
        while let Some(tail_key) = self.small_fifo.pop_back() {
            let Some(tail) = self.values.get(&tail_key) else {
                continue;
            };

            self.small_count -= 1;
            self.small_size -= tail.size;

            if tail.freq > 1 {
                let size = tail.size;
                self.main_fifo.push_back(tail_key);
                self.main_count += 1;
                self.main_size += size;

                while self.main_count > self.max_main_count || self.main_size > self.max_main_size {
                    self.evict_m();
                }
            } else {
                let _ = self.values.remove(&tail_key).unwrap();

                // Insert into ghost list (it will handle size limits internally)
                self.ghost.insert(tail_key);

                return;
            }
        }
    }

    fn evict_m(&mut self) {
        while let Some(tail_key) = self.main_fifo.pop_back() {
            let Some(tail) = self.values.get_mut(&tail_key) else {
                continue;
            };

            self.main_count -= 1;
            self.main_size -= tail.size;

            if tail.freq > 0 {
                self.main_count += 1;
                self.main_size += tail.size;
                tail.freq = tail.freq.saturating_sub(1);
                self.main_fifo.push_front(tail_key);
            } else {
                let _ = self.values.remove(&tail_key).unwrap();
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::num::{NonZeroU32, NonZeroUsize};

    use super::*;

    #[test]
    fn test_ghost_list_basic_operations() {
        let mut ghost = GhostList::new(3);

        assert_eq!(ghost.len(), 0);
        assert!(!ghost.contains("key1"));

        ghost.insert("key1".to_string());
        assert_eq!(ghost.len(), 1);
        assert!(ghost.contains("key1"));

        ghost.insert("key2".to_string());
        ghost.insert("key3".to_string());
        assert_eq!(ghost.len(), 3);

        ghost.insert("key1".to_string());
        assert_eq!(ghost.len(), 3);

        ghost.remove("key2");
        assert_eq!(ghost.len(), 2);
        assert!(!ghost.contains("key2"));
        assert!(ghost.contains("key1"));
        assert!(ghost.contains("key3"));
    }

    #[test]
    fn test_ghost_list_fifo_eviction() {
        let mut ghost = GhostList::new(2);

        ghost.insert("first".to_string());
        ghost.insert("second".to_string());
        assert_eq!(ghost.len(), 2);

        ghost.insert("third".to_string());
        assert_eq!(ghost.len(), 2);
        assert!(!ghost.contains("first"));
        assert!(ghost.contains("second"));
        assert!(ghost.contains("third"));

        ghost.insert("fourth".to_string());
        assert_eq!(ghost.len(), 2);
        assert!(!ghost.contains("second"));
        assert!(ghost.contains("third"));
        assert!(ghost.contains("fourth"));
    }

    #[test]
    fn test_ghost_list_should_compact() {
        let mut ghost = GhostList::new(4);

        ghost.insert("a".to_string());
        ghost.insert("b".to_string());
        ghost.insert("c".to_string());
        ghost.insert("d".to_string());

        ghost.remove("b");
        ghost.remove("c");
        assert_eq!(ghost.len(), 2);

        assert_eq!(ghost.queue.len(), 4);

        // Should not need compaction yet (50% threshold).
        assert!(!ghost.should_compact());

        // Remove one more to trigger should_compact.
        ghost.remove("d");
        assert_eq!(ghost.len(), 1);

        assert!(ghost.should_compact());
    }

    #[test]
    fn test_ghost_list_evict_oldest_with_tombstones() {
        let mut ghost = GhostList::new(3);

        ghost.insert("a".to_string());
        ghost.insert("b".to_string());
        ghost.insert("c".to_string());
        ghost.insert("d".to_string());

        assert_eq!(ghost.len(), 3);
        assert!(!ghost.contains("a"));

        ghost.remove("b");
        ghost.remove("c");
        assert_eq!(ghost.len(), 1);

        // Now evict_oldest should skip tombstones and evict 'd'.
        let evicted = ghost.evict_oldest();
        assert_eq!(evicted, Some("d".to_string()));
        assert_eq!(ghost.len(), 0);
        assert_eq!(ghost.queue.len(), 0);
    }

    #[test]
    fn test_ghost_list_compact() {
        let mut ghost = GhostList::new(5);

        for i in 0..5 {
            ghost.insert(format!("key{i}"));
        }

        ghost.remove("key1");
        ghost.remove("key3");
        ghost.remove("key2");

        assert_eq!(ghost.len(), 2);
        assert_eq!(ghost.queue.len(), 5);

        assert!(ghost.should_compact());
        ghost.compact();

        assert_eq!(ghost.len(), 2);
        assert_eq!(ghost.queue.len(), 2);

        assert!(ghost.contains("key0"));
        assert!(!ghost.contains("key2"));
        assert!(ghost.contains("key4"));

        assert_eq!(ghost.evict_oldest(), Some("key0".to_string()));
        assert_eq!(ghost.evict_oldest(), Some("key4".to_string()));
        assert_eq!(ghost.evict_oldest(), None);
    }

    #[derive(Debug, Clone, PartialEq)]
    struct TestData {
        size: usize,
    }

    impl TestData {
        fn new(size: usize) -> Self {
            Self { size }
        }
    }

    impl Cacheable for TestData {
        fn size(&self) -> usize {
            self.size
        }
    }

    #[test]
    fn test_basic_insertion_and_retrieval() {
        let mut cache: SimpleCache<String, TestData> = SimpleCache::new(NonZeroU32::new(100).unwrap(), NonZeroUsize::new(10000).unwrap());

        let key1 = "test_key_1".to_string();
        let data1 = TestData::new(500);

        assert!(cache.insert(key1.clone(), data1.clone()).is_ok());
        assert_eq!(cache.count(), 1);
        assert_eq!(cache.size(), 500);

        let retrieved = cache.get(&key1);
        assert!(retrieved.is_some());
        assert_eq!(*retrieved.unwrap(), data1);
    }

    #[test]
    fn test_multiple_insertions() {
        let mut cache: SimpleCache<String, TestData> = SimpleCache::new(NonZeroU32::new(100).unwrap(), NonZeroUsize::new(10000).unwrap());

        for i in 0..50 {
            let key = format!("key_{i}");
            let data = TestData::new(100);
            assert!(cache.insert(key.clone(), data).is_ok());
        }

        assert_eq!(cache.count(), 10);
        assert_eq!(cache.size(), 1000);

        for i in 0..10 {
            let key = format!("key_{i}");
            let data = TestData::new(100);
            assert!(cache.insert(key.clone(), data).is_ok());
        }

        assert_eq!(cache.count(), 20);
        assert_eq!(cache.size(), 2000);

        // Ghosts are promoted to main FIFO.
        for i in 0..10 {
            let key = format!("key_{i}");
            assert!(cache.get(&key).is_some(), "Key {key} should be present");
        }

        // The last batch of one hits are still in small FIFO.
        for i in 40..50 {
            let key = format!("key_{i}");
            assert!(cache.get(&key).is_some(), "Key {key} should be present");
        }
    }

    #[test]
    fn test_cache_eviction_by_count() {
        let mut cache: SimpleCache<String, TestData> = SimpleCache::new(NonZeroU32::new(100).unwrap(), NonZeroUsize::new(100000).unwrap());

        for i in 0..20 {
            let key = format!("key_{i}");
            let data = TestData::new(100);
            assert!(cache.insert(key.clone(), data).is_ok());
        }

        assert_eq!(cache.count(), 10);

        for i in 10..20 {
            let key = format!("key_{i}");
            assert!(cache.get(&key).is_some(), "Key {key} should be present");
        }
    }

    #[test]
    fn test_cache_eviction_by_size() {
        let mut cache: SimpleCache<String, TestData> = SimpleCache::new(NonZeroU32::new(100).unwrap(), NonZeroUsize::new(10000).unwrap());

        for i in 0..10 {
            let key = format!("key_{i}");
            let data = TestData::new(500);
            assert!(cache.insert(key.clone(), data).is_ok());
        }

        assert_eq!(cache.size(), 1000);

        for i in 8..10 {
            let key = format!("key_{i}");
            assert!(cache.get(&key).is_some(), "Key {key} should be present");
        }
    }

    #[test]
    fn test_value_too_big_error() {
        let mut cache: SimpleCache<String, TestData> = SimpleCache::new(NonZeroU32::new(100).unwrap(), NonZeroUsize::new(5000).unwrap());

        let key = "big_item".to_string();
        let big_data = TestData::new(6000);

        let result = cache.insert(key, big_data);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CacheError::ValueTooBig));

        assert_eq!(cache.count(), 0);
        assert_eq!(cache.size(), 0);
    }

    #[test]
    fn test_overwrite_existing_key() {
        let mut cache: SimpleCache<String, TestData> = SimpleCache::new(NonZeroU32::new(100).unwrap(), NonZeroUsize::new(10000).unwrap());

        let key = "overwrite_test".to_string();

        let data1 = TestData::new(1000);
        assert!(cache.insert(key.clone(), data1.clone()).is_ok());

        let data2 = TestData::new(1500);
        assert_eq!(cache.insert(key.clone(), data2.clone()), Err(CacheError::ValueAlreadyPresent));
    }
}
