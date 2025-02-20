//! Implements a generational slab.

use std::iter::Enumerate;
use std::marker::PhantomData;
use std::mem::swap;
use std::num::NonZeroU32;
use std::slice::Iter;

/// Trait for keys of generation slabs.
pub trait GenerationalKey: Copy {
    #[doc(hidden)]
    /// Creates a new fixed key. Must not be called by the user.
    fn new(key: u32, generation: NonZeroU32) -> Self;
    #[doc(hidden)]
    /// Returns the key value.
    fn key(&self) -> u32;
    #[doc(hidden)]
    /// Returns the generation.
    fn generation(&self) -> NonZeroU32;
}

enum Slot<T> {
    Occupied {
        value: T,
        generation: NonZeroU32,
    },
    Empty {
        next_free: Option<u32>,
        last_generation: NonZeroU32,
    },
}

/// A slab with generational slots. Can have at most [`u32::MAX`] entries.
pub struct GenerationalSlab<I, V> {
    entries: Vec<Slot<V>>,
    next_free: Option<u32>,
    _marker: PhantomData<I>,
}

impl<I: GenerationalKey, V> Default for GenerationalSlab<I, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<I: GenerationalKey, V> GenerationalSlab<I, V> {
    /// Creates a new generational slab.
    pub fn new() -> Self {
        Self {
            entries: Vec::default(),
            next_free: None,
            _marker: PhantomData,
        }
    }

    /// Creates a new generational slab with the given pre-allocated capacity.
    pub fn with_capacity(size: u32) -> Self {
        Self {
            entries: Vec::with_capacity(size as usize),
            next_free: None,
            _marker: PhantomData,
        }
    }

    /// Inserts a new value into the slab. Returns the key of the value if
    /// there was still space left for the value inside the slab.
    #[must_use]
    pub fn insert(&mut self, value: V) -> Option<I> {
        if let Some(key) = self.next_free
            && let Some(Slot::Empty {
                next_free,
                last_generation,
            }) = self.entries.get(key as usize)
        {
            self.next_free = *next_free;

            let generation = match last_generation.get().checked_add(1) {
                None => NonZeroU32::new(1).expect("one is zero"),
                Some(value) => NonZeroU32::new(value).expect("value is zero"),
            };

            self.entries[key as usize] = Slot::Occupied { value, generation };

            Some(I::new(key, generation))
        } else if self.entries.len() < u32::MAX as usize {
            let generation = NonZeroU32::new(1).expect("one is zero");
            let key = self.entries.len();

            self.entries.push(Slot::Occupied { value, generation });

            let key = u32::try_from(key).expect("key is not an u32");
            Some(I::new(key, generation))
        } else {
            None
        }
    }

    /// Returns a reference to the value of the given key.
    #[must_use]
    pub fn get(&self, key: I) -> Option<&V> {
        if let Some(Slot::Occupied { value, generation }) = self.entries.get(key.key() as usize)
            && key.generation() == *generation
        {
            return Some(value);
        }

        None
    }

    /// Returns a mutable reference to the value of the given key.
    #[must_use]
    pub fn get_mut(&mut self, key: I) -> Option<&mut V> {
        if let Some(Slot::Occupied { value, generation }) = self.entries.get_mut(key.key() as usize)
            && key.generation() == *generation
        {
            return Some(value);
        }

        None
    }

    /// Iterates over all non-empty entries.
    #[must_use]
    pub fn iter(&self) -> GenerationalIter<I, V> {
        GenerationalIter {
            entries: self.entries.iter().enumerate(),
            size: self.entries.len(),
            _marker: PhantomData,
        }
    }

    /// Removes the value with the given key if present.
    #[must_use]
    pub fn remove(&mut self, key: I) -> Option<V> {
        if let Some(entry) = self.entries.get_mut(key.key() as usize)
            && let Slot::Occupied { generation, .. } = entry
            && key.generation() == *generation
        {
            let mut empty_slot = Slot::Empty {
                next_free: self.next_free,
                last_generation: *generation,
            };

            swap(&mut empty_slot, entry);
            self.next_free = Some(key.key());

            if let Slot::Occupied { value, .. } = empty_slot {
                return Some(value);
            }
        }

        None
    }

    /// Clears the slab.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.next_free = None;
    }
}

struct SecondarySlot<T> {
    value: T,
    generation: NonZeroU32,
}

/// A secondary slab with generational slots. Re-uses the key from another
/// [`GenerationalSlab`].
pub struct SecondaryGenerationalSlab<I, V> {
    entries: Vec<Option<SecondarySlot<V>>>,
    _marker: PhantomData<I>,
}

impl<I: GenerationalKey, V> Default for SecondaryGenerationalSlab<I, V> {
    fn default() -> Self {
        Self {
            entries: Vec::default(),
            _marker: PhantomData,
        }
    }
}

impl<I: GenerationalKey, V> SecondaryGenerationalSlab<I, V> {
    /// Inserts a value at the given key from a [`GenerationSlab`].
    pub fn insert(&mut self, key: I, value: V) {
        if key.key() as usize >= self.entries.len() {
            self.entries.resize_with((key.key() as usize) + 1, || None)
        }

        if let Some(Some(value)) = self.entries.get_mut(key.key() as usize)
            && key.generation() < value.generation
        {
            return;
        }

        self.entries[key.key() as usize] = Some(SecondarySlot {
            value,
            generation: key.generation(),
        });
    }

    /// Returns true if the slot of the key is occupied.
    #[must_use]
    pub fn contains_key(&self, key: I) -> bool {
        match self.entries.get(key.key() as usize) {
            Some(Some(slot)) => key.generation() == slot.generation,
            _ => false,
        }
    }

    /// Returns the value at the key.
    #[must_use]
    pub fn get(&self, key: I) -> Option<&V> {
        if let Some(slot) = self.entries.get(key.key() as usize)?
            && key.generation() == slot.generation
        {
            return Some(&slot.value);
        }

        None
    }

    /// Returns a mutable reference to the value at the key.
    #[must_use]
    pub fn get_mut(&mut self, key: I) -> Option<&mut V> {
        if let Some(slot) = self.entries.get_mut(key.key() as usize)?
            && key.generation() == slot.generation
        {
            return Some(&mut slot.value);
        }

        None
    }

    /// Removes and returns the value at the given key.
    #[must_use]
    pub fn remove(&mut self, key: I) -> Option<V> {
        let entry = self.entries.get_mut(key.key() as usize)?;

        if let Some(slot) = entry.as_ref()
            && key.generation() != slot.generation
        {
            return None;
        }

        let mut empty_option = None;
        swap(entry, &mut empty_option);
        let slot = empty_option.unwrap();

        (key.generation() == slot.generation).then_some(slot.value)
    }
}

/// Iterator over all non-empty entry slots.
pub struct GenerationalIter<'a, I, T: 'a> {
    entries: Enumerate<Iter<'a, Slot<T>>>,
    size: usize,
    _marker: PhantomData<I>,
}

impl<'a, I: GenerationalKey, T> Iterator for GenerationalIter<'a, I, T> {
    type Item = (I, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.entries.next() {
                Some((index, Slot::Occupied { value, generation })) => return Some((I::new(index as u32, *generation), value)),
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
    #![allow(clippy::unwrap_used)]

    use crate::container::{GenerationalKey, GenerationalSlab, SecondaryGenerationalSlab};

    create_generational_key!(TestKey);

    #[test]
    fn test_generational_key() {
        let mut slab: GenerationalSlab<TestKey, u32> = GenerationalSlab::default();
        let key_0 = slab.insert(0).unwrap();
        let key_1 = slab.insert(1).unwrap();
        let key_2 = slab.insert(2).unwrap();

        assert_eq!(key_0.key, 0);
        assert_eq!(key_0.generation.get(), 1);
        assert_eq!(key_1.key, 1);
        assert_eq!(key_1.generation.get(), 1);
        assert_eq!(key_2.key, 2);
        assert_eq!(key_2.generation.get(), 1);

        assert!(slab.remove(key_0).is_some());
        let key_0 = slab.insert(42).unwrap();

        assert_eq!(key_0.key, 0);
        assert_eq!(key_0.generation.get(), 2);
    }

    #[test]
    fn test_generational_insert_updates() {
        let mut slab: GenerationalSlab<TestKey, u32> = GenerationalSlab::default();

        let key_0 = slab.insert(0).unwrap();
        assert!(slab.remove(key_0).is_some());
        let key_0 = slab.insert(42).unwrap();

        assert_eq!(*slab.get(key_0).unwrap(), 42);
    }

    #[test]
    fn test_generational_get_mut() {
        let mut slab: GenerationalSlab<TestKey, u32> = GenerationalSlab::default();

        let key_0 = slab.insert(0).unwrap();
        *slab.get_mut(key_0).unwrap() = 13;

        assert_eq!(*slab.get(key_0).unwrap(), 13);
    }

    #[test]
    fn test_generational_old_key() {
        let mut slab: GenerationalSlab<TestKey, u32> = GenerationalSlab::default();
        let key_0 = slab.insert(2).unwrap();
        assert!(slab.remove(key_0).is_some());
        let key_1 = slab.insert(3).unwrap();

        assert!(slab.get(key_1).is_some());
        assert!(slab.get_mut(key_1).is_some());

        assert!(slab.get(key_0).is_none());
        assert!(slab.get_mut(key_0).is_none());

        assert!(slab.remove(key_0).is_none());
        assert!(slab.get(key_1).is_some());
        assert!(slab.get_mut(key_1).is_some());

        assert!(slab.remove(key_1).is_some());
        assert!(slab.get(key_0).is_none());
        assert!(slab.get_mut(key_0).is_none());
    }

    #[test]
    fn test_generational_slab_iterator() {
        let mut slab: GenerationalSlab<TestKey, u32> = GenerationalSlab::default();
        let keys: Vec<TestKey> = (0..10).map(|i| slab.insert(i).unwrap()).collect();

        let mut iter_count = 0;
        for (key, value) in slab.iter() {
            assert!(keys.contains(&key));
            assert_eq!(*value, key.key());
            iter_count += 1;
        }
        assert_eq!(iter_count, 10);
    }

    #[test]
    fn test_generational_slab_clear() {
        let mut slab: GenerationalSlab<TestKey, u32> = GenerationalSlab::default();
        for index in 0..10 {
            let _ = slab.insert(index);
        }
        assert_eq!(slab.iter().count(), 10);
        slab.clear();
        assert_eq!(slab.iter().count(), 0);
        assert!(slab.insert(0).is_some());
    }

    #[test]
    fn test_secondary_insert() {
        let mut primary: GenerationalSlab<TestKey, u32> = GenerationalSlab::default();
        let mut secondary: SecondaryGenerationalSlab<TestKey, u32> = SecondaryGenerationalSlab::default();
        let key_0 = primary.insert(12).unwrap();
        let key_1 = primary.insert(14).unwrap();

        secondary.insert(key_0, 89);
        secondary.insert(key_1, 91);

        assert!(secondary.contains_key(key_0));
        assert!(secondary.contains_key(key_1));

        assert_eq!(*secondary.get(key_0).unwrap(), 89);
        assert_eq!(*secondary.get(key_1).unwrap(), 91);
    }

    #[test]
    fn test_secondary_expanding_insert() {
        let mut primary: GenerationalSlab<TestKey, u32> = GenerationalSlab::default();
        let mut secondary: SecondaryGenerationalSlab<TestKey, u32> = SecondaryGenerationalSlab::default();
        let key_0 = primary.insert(2).unwrap();
        let key_1 = primary.insert(3).unwrap();

        secondary.insert(key_1, 12);

        assert!(!secondary.contains_key(key_0));
        assert!(secondary.contains_key(key_1));

        assert_eq!(secondary.get(key_0), None);
        assert_eq!(*secondary.get(key_1).unwrap(), 12);
    }

    #[test]
    fn test_secondary_get_mut() {
        let mut primary: GenerationalSlab<TestKey, u32> = GenerationalSlab::default();
        let mut secondary: SecondaryGenerationalSlab<TestKey, u32> = SecondaryGenerationalSlab::default();

        let key = primary.insert(10).unwrap();
        secondary.insert(key, 20);

        if let Some(value) = secondary.get_mut(key) {
            *value = 30;
        }

        assert_eq!(*secondary.get(key).unwrap(), 30);
    }

    #[test]
    fn test_secondary_old_key() {
        let mut primary: GenerationalSlab<TestKey, u32> = GenerationalSlab::default();
        let mut secondary: SecondaryGenerationalSlab<TestKey, u32> = SecondaryGenerationalSlab::default();
        let key_0 = primary.insert(2).unwrap();
        assert!(primary.remove(key_0).is_some());
        let key_1 = primary.insert(3).unwrap();

        secondary.insert(key_1, 13);
        assert!(secondary.contains_key(key_1));
        assert!(secondary.get(key_1).is_some());

        secondary.insert(key_0, 12);
        assert!(!secondary.contains_key(key_0));
        assert!(secondary.get(key_0).is_none());
    }

    #[test]
    fn test_secondary_insert_invalid_key() {
        let mut primary: GenerationalSlab<TestKey, u32> = GenerationalSlab::default();
        let mut secondary: SecondaryGenerationalSlab<TestKey, u32> = SecondaryGenerationalSlab::default();

        let key = primary.insert(10).unwrap();
        let _ = primary.remove(key);
        let new_key = primary.insert(20).unwrap();

        secondary.insert(key, 30);
        secondary.insert(new_key, 40);

        assert!(!secondary.contains_key(key));
        assert!(secondary.contains_key(new_key));
        assert_eq!(*secondary.get(new_key).unwrap(), 40);
    }

    #[test]
    fn test_secondary_remove() {
        let mut primary: GenerationalSlab<TestKey, u32> = GenerationalSlab::default();
        let mut secondary: SecondaryGenerationalSlab<TestKey, u32> = SecondaryGenerationalSlab::default();
        let key_0 = primary.insert(2).unwrap();
        assert!(primary.remove(key_0).is_some());
        let key_1 = primary.insert(3).unwrap();

        secondary.insert(key_1, 13);
        assert!(secondary.contains_key(key_1));
        assert!(secondary.get(key_1).is_some());

        assert!(secondary.remove(key_0).is_none());
        assert!(secondary.remove(key_1).is_some());
    }
}
