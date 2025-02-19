//! A fixed sized slab allocator.

use std::iter::Enumerate;
use std::marker::PhantomData;
use std::mem::swap;
use std::slice::Iter;

/// Trait for keys of simple slabs.
pub trait SimpleKey: Copy {
    #[doc(hidden)]
    /// Creates a new key. Must not be called by the user.
    fn new(key: u32) -> Self;
    #[doc(hidden)]
    /// Returns the key value.
    fn key(&self) -> u32;
}

impl SimpleKey for u32 {
    fn new(key: u32) -> Self {
        key
    }

    fn key(&self) -> u32 {
        *self
    }
}

enum Slot<T> {
    Occupied(T),
    Empty(Option<u32>),
}

/// A simple slab container. Can have at most [`u32::MAX`] entries.
pub struct SimpleSlab<I, T> {
    entries: Vec<Slot<T>>,
    next_free: Option<u32>,
    count: u32,
    _marker: PhantomData<I>,
}

impl<I: SimpleKey, T> Default for SimpleSlab<I, T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<I: SimpleKey, T> SimpleSlab<I, T> {
    /// Creates a new simple slab.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Vec::default(),
            next_free: None,
            count: 0,
            _marker: PhantomData,
        }
    }

    /// Creates a new simple slab that has the given capacity.
    #[must_use]
    pub fn with_capacity(capacity: u32) -> Self {
        Self {
            entries: Vec::with_capacity(capacity as usize),
            next_free: None,
            count: 0,
            _marker: PhantomData,
        }
    }

    /// Returns a reference if the key is present.
    #[must_use]
    pub fn get(&self, key: I) -> Option<&T> {
        let key = key.key() as usize;

        if let Slot::Occupied(value) = self.entries.get(key)? {
            Some(value)
        } else {
            None
        }
    }

    /// Returns a mutable reference if the key is present.
    #[must_use]
    pub fn get_mut(&mut self, key: I) -> Option<&mut T> {
        let key = key.key() as usize;

        if let Slot::Occupied(value) = self.entries.get_mut(key)? {
            Some(value)
        } else {
            None
        }
    }

    /// Inserts the value into the slab. Returns the key or `None` if the slab
    /// is full.
    #[must_use]
    pub fn insert(&mut self, value: T) -> Option<I> {
        if let Some(key) = self.next_free
            && let Some(Slot::Empty(next_free)) = self.entries.get(key as usize)
        {
            self.next_free = *next_free;

            self.entries[key as usize] = Slot::Occupied(value);
            self.count += 1;

            Some(I::new(key))
        } else if self.entries.len() < u32::MAX as usize {
            let key = self.entries.len();

            self.entries.push(Slot::Occupied(value));
            self.count += 1;

            let key = u32::try_from(key).expect("key is not an u32");
            Some(I::new(key))
        } else {
            None
        }
    }

    /// Removes the value for the given key.
    #[must_use]
    pub fn remove(&mut self, key: I) -> Option<T> {
        let index = key.key() as usize;
        let entry = self.entries.get_mut(index)?;

        if let Slot::Empty(_) = entry {
            return None;
        }

        let mut empty_slot = Slot::Empty(self.next_free);
        swap(entry, &mut empty_slot);
        self.next_free = Some(key.key());

        if let Slot::Occupied(value) = empty_slot {
            self.count -= 1;
            return Some(value);
        }

        None
    }

    /// Iterates over all non-empty entries.
    #[must_use]
    pub fn iter(&self) -> SimpleIterator<I, T> {
        SimpleIterator {
            entries: self.entries.iter().enumerate(),
            size: self.entries.len(),
            _marker: PhantomData,
        }
    }

    /// Removes all elements from the slab, returning them as an iterator.
    pub fn drain(&mut self) -> DrainIter<I, T> {
        let old_len = self.entries.len();
        self.next_free = None;
        self.count = 0;
        DrainIter {
            slab: self,
            index: 0,
            len: old_len,
        }
    }

    /// Returns the amount of occupied slab entries.
    pub fn count(&self) -> u32 {
        self.count
    }

    /// Clears the slab.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.next_free = None;
        self.count = 0;
    }
}

pub struct DrainIter<'a, I: SimpleKey, T> {
    slab: &'a mut SimpleSlab<I, T>,
    index: usize,
    len: usize,
}

impl<'a, I: SimpleKey, T> Iterator for DrainIter<'a, I, T> {
    type Item = (I, T);

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.len {
            let current_index = self.index;
            self.index += 1;

            if let Slot::Occupied(value) = std::mem::replace(&mut self.slab.entries[current_index], Slot::Empty(None)) {
                return Some((I::new(current_index as u32), value));
            }
        }
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.len - self.index))
    }
}

impl<'a, I: SimpleKey, T> Drop for DrainIter<'a, I, T> {
    fn drop(&mut self) {
        // Exhaust the iterator to ensure all remaining elements are dropped.
        for _ in self {}
    }
}

/// A secondary slab with generational slots. Re-uses the key from another
/// [`SimpleSlab`].
#[derive(Clone)]
pub struct SecondarySimpleSlab<I, V> {
    entries: Vec<Option<V>>,
    _marker: PhantomData<I>,
}

impl<I, V> SecondarySimpleSlab<I, V> {
    /// Creates a new [`SecondarySimpleSlab`] with the given capacity.
    pub fn with_capacity(capacity: u32) -> Self {
        Self {
            entries: Vec::with_capacity(capacity as usize),
            _marker: PhantomData,
        }
    }
}

impl<I: SimpleKey, V> Default for SecondarySimpleSlab<I, V> {
    fn default() -> Self {
        Self {
            entries: Vec::default(),
            _marker: PhantomData,
        }
    }
}

impl<I: SimpleKey, V> SecondarySimpleSlab<I, V> {
    /// Inserts a value at the given key from a [`SimpleSlab`].
    pub fn insert(&mut self, key: I, value: V) {
        if key.key() as usize >= self.entries.len() {
            self.entries.resize_with((key.key() as usize) + 1, || None)
        }

        self.entries[key.key() as usize] = Some(value);
    }

    /// Returns true if the slot of the key is occupied.
    #[must_use]
    pub fn contains_key(&self, key: I) -> bool {
        self.entries.get(key.key() as usize).is_some()
    }

    /// Returns the value at the key.
    #[must_use]
    pub fn get(&self, key: I) -> Option<&V> {
        self.entries.get(key.key() as usize).and_then(|slot| slot.as_ref())
    }

    /// Returns a mutable reference to the value at the key.
    #[must_use]
    pub fn get_mut(&mut self, key: I) -> Option<&mut V> {
        self.entries.get_mut(key.key() as usize).and_then(|slot| slot.as_mut())
    }

    /// Removes and returns the value at the given key.
    #[must_use]
    pub fn remove(&mut self, key: I) -> Option<V> {
        if let Some(entry) = self.entries.get_mut(key.key() as usize)
            && entry.as_ref().is_some()
        {
            let mut empty_option = None;
            swap(entry, &mut empty_option);
            let value = empty_option.unwrap();
            return Some(value);
        }

        None
    }

    /// Clears the slab.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Iterates over all non-empty entries.
    #[must_use]
    pub fn iter(&self) -> SecondarySimpleIterator<I, V> {
        SecondarySimpleIterator {
            entries: self.entries.iter().enumerate(),
            size: self.entries.len(),
            _marker: PhantomData,
        }
    }
}

/// Iterator over all non-empty entry slots.
pub struct SimpleIterator<'a, I, T: 'a> {
    entries: Enumerate<Iter<'a, Slot<T>>>,
    size: usize,
    _marker: PhantomData<I>,
}

impl<'a, I: SimpleKey, T> Iterator for SimpleIterator<'a, I, T> {
    type Item = (I, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.entries.next() {
                Some((index, Slot::Occupied(value))) => return Some((I::new(index as u32), value)),
                Some(_) => continue,
                None => return None,
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.size))
    }
}

/// Iterator over all non-empty entry slots.
pub struct SecondarySimpleIterator<'a, I, T: 'a> {
    entries: Enumerate<Iter<'a, Option<T>>>,
    size: usize,
    _marker: PhantomData<I>,
}

impl<'a, I: SimpleKey, T> Iterator for SecondarySimpleIterator<'a, I, T> {
    type Item = (I, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.entries.next() {
                Some((index, Some(value))) => return Some((I::new(index as u32), value)),
                Some(_) => continue,
                None => return None,
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.size))
    }
}

#[cfg(test)]
mod tests {
    use crate::container::{SecondarySimpleSlab, SimpleSlab};

    #[test]
    fn test_simple_slab_insert_and_get() {
        let mut slab = SimpleSlab::<u32, i32>::new();

        let key_1 = slab.insert(10).unwrap();
        let key_2 = slab.insert(20).unwrap();
        let key_3 = slab.insert(30).unwrap();

        assert_eq!(slab.get(key_1), Some(&10));
        assert_eq!(slab.get(key_2), Some(&20));
        assert_eq!(slab.get(key_3), Some(&30));
        assert_eq!(slab.count(), 3);
    }

    #[test]
    fn test_simple_slab_remove() {
        let mut slab = SimpleSlab::<u32, i32>::new();

        let key_1 = slab.insert(10).unwrap();
        let key_2 = slab.insert(20).unwrap();

        assert_eq!(slab.remove(key_1), Some(10));
        assert_eq!(slab.get(key_1), None);
        assert_eq!(slab.count(), 1);

        assert_eq!(slab.remove(key_2), Some(20));
        assert_eq!(slab.get(key_2), None);
        assert_eq!(slab.count(), 0);
    }

    #[test]
    fn test_simple_slab_reuse_slots() {
        let mut slab = SimpleSlab::<u32, i32>::new();

        let key_1 = slab.insert(10).unwrap();
        let _ = slab.insert(20).unwrap();

        let _ = slab.remove(key_1);
        let key_3 = slab.insert(30).unwrap();

        assert_eq!(key_1, key_3);
        assert_eq!(slab.get(key_3), Some(&30));
    }

    #[test]
    fn test_simple_slab_iter() {
        let mut slab = SimpleSlab::<u32, i32>::new();

        let _ = slab.insert(10);
        let _ = slab.insert(20);
        let _ = slab.insert(30);

        let values: Vec<(u32, &i32)> = slab.iter().collect();
        assert_eq!(values, vec![(0, &10), (1, &20), (2, &30)]);
    }

    #[test]
    fn test_simple_slab_clear() {
        let mut slab = SimpleSlab::<u32, i32>::new();

        let _ = slab.insert(10);
        let _ = slab.insert(20);
        let _ = slab.insert(30);

        assert_eq!(slab.count(), 3);

        slab.clear();

        assert_eq!(slab.count(), 0);
        assert_eq!(slab.iter().count(), 0);
    }

    #[test]
    fn test_simple_slab_with_capacity() {
        let slab = SimpleSlab::<u32, i32>::with_capacity(10);
        assert!(slab.entries.capacity() >= 10);
    }

    #[test]
    fn test_simple_slab_get_mut() {
        let mut slab = SimpleSlab::<u32, i32>::new();

        let key = slab.insert(10).unwrap();

        if let Some(value) = slab.get_mut(key) {
            *value += 5;
        }

        assert_eq!(slab.get(key), Some(&15));
    }

    #[test]
    fn test_simple_slab_remove_nonexistent() {
        let mut slab = SimpleSlab::<u32, i32>::new();
        let key = slab.insert(10).unwrap();
        assert_eq!(slab.remove(key + 1), None);
    }

    #[test]
    fn test_secondary_simple_slab_insert_and_get() {
        let mut primary = SimpleSlab::<u32, i32>::new();
        let mut secondary = SecondarySimpleSlab::<u32, String>::default();

        let key_1 = primary.insert(10).unwrap();
        let key_2 = primary.insert(20).unwrap();

        secondary.insert(key_1, "Hello".to_string());
        secondary.insert(key_2, "World".to_string());

        assert_eq!(secondary.get(key_1), Some(&"Hello".to_string()));
        assert_eq!(secondary.get(key_2), Some(&"World".to_string()));
    }

    #[test]
    fn test_secondary_simple_slab_contains_key() {
        let mut primary = SimpleSlab::<u32, i32>::new();
        let mut secondary = SecondarySimpleSlab::<u32, String>::default();

        let key_1 = primary.insert(10).unwrap();
        let key_2 = primary.insert(20).unwrap();

        secondary.insert(key_1, "Hello".to_string());

        assert!(secondary.contains_key(key_1));
        assert!(!secondary.contains_key(key_2));
    }

    #[test]
    fn test_secondary_simple_slab_get_mut() {
        let mut primary = SimpleSlab::<u32, i32>::new();
        let mut secondary = SecondarySimpleSlab::<u32, String>::default();

        let key = primary.insert(10).unwrap();
        secondary.insert(key, "Hello".to_string());

        if let Some(value) = secondary.get_mut(key) {
            value.push_str(" World");
        }

        assert_eq!(secondary.get(key), Some(&"Hello World".to_string()));
    }

    #[test]
    fn test_secondary_simple_slab_remove() {
        let mut primary = SimpleSlab::<u32, i32>::new();
        let mut secondary = SecondarySimpleSlab::<u32, String>::default();

        let key_1 = primary.insert(10).unwrap();
        let key_2 = primary.insert(20).unwrap();

        secondary.insert(key_1, "Hello".to_string());
        secondary.insert(key_2, "World".to_string());

        assert_eq!(secondary.remove(key_1), Some("Hello".to_string()));
        assert_eq!(secondary.get(key_1), None);
        assert_eq!(secondary.remove(key_2), Some("World".to_string()));
        assert_eq!(secondary.get(key_2), None);
    }

    #[test]
    fn test_secondary_simple_slab_remove_nonexistent() {
        let mut primary = SimpleSlab::<u32, i32>::new();
        let mut secondary = SecondarySimpleSlab::<u32, String>::default();

        let key = primary.insert(10).unwrap();
        assert_eq!(secondary.remove(key), None);
    }

    #[test]
    fn test_simple_slab_iter_size_hint() {
        let mut slab = SimpleSlab::<u32, i32>::new();
        for index in 0..5 {
            let _ = slab.insert(index);
        }
        let iterator = slab.iter();
        assert_eq!(iterator.size_hint(), (0, Some(5)));
    }
}
