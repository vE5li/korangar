//! Implements a sized Lru cache.

use std::alloc::{alloc, dealloc, Layout};
use std::boxed::Box;
use std::marker::PhantomData;
use std::num::NonZeroU32;
use std::ops::Drop;
use std::ptr;
use std::ptr::NonNull;

use super::generational_slab::SecondaryGenerationalSlab;
use super::GenerationalKey;

struct Entry<I, V> {
    key: Option<I>,
    value: Option<V>,
    next: *mut Entry<I, V>,
    prev: *mut Entry<I, V>,
    size: usize,
}

impl<I, V> Entry<I, V> {
    fn new_zeroed() -> Self {
        Entry {
            key: None,
            value: None,
            prev: ptr::null_mut(),
            next: ptr::null_mut(),
            size: 0,
        }
    }
}

/// An LRU cache strategy that tracks the usage of its items and the overall
/// size they represent.
pub struct Lru<I, V> {
    pointer: NonNull<Entry<I, V>>,
    entries: SecondaryGenerationalSlab<I, *mut Entry<I, V>>,
    list: *mut Entry<I, V>,
    free: *mut Entry<I, V>,
    size: usize,
    count: u32,
    max_count: NonZeroU32,
    _marker: PhantomData<Entry<I, V>>,
}

unsafe impl<I, V> Send for Lru<I, V> {}

impl<I, V> Drop for Lru<I, V> {
    fn drop(&mut self) {
        unsafe {
            let layout = Layout::array::<Entry<I, V>>(self.max_count.get() as usize).expect("invalid layout");
            dealloc(self.pointer.as_ptr().cast(), layout);
            drop(Box::from_raw(self.list));
            drop(Box::from_raw(self.free));
        }
    }
}

// Any changes to this structs implementation should be tested with miri for
// soundness.
impl<I: GenerationalKey + Copy, V> Lru<I, V> {
    pub(crate) fn new(max_count: NonZeroU32) -> Self {
        let layout = Layout::array::<Entry<I, V>>(max_count.get() as usize).expect("invalid layout");
        assert!(layout.size() < isize::MAX as usize);

        let pointer = unsafe { alloc(layout).cast() };
        let pointer = NonNull::new(pointer).expect("out of memory");

        let mut cache = Self {
            pointer,
            entries: SecondaryGenerationalSlab::default(),
            list: Box::into_raw(Box::new(Entry::new_zeroed())),
            free: Box::into_raw(Box::new(Entry::new_zeroed())),
            size: 0,
            count: 0,
            max_count,
            _marker: PhantomData,
        };

        unsafe {
            (*cache.list).next = cache.list;
            (*cache.list).prev = cache.list;
            (*cache.free).next = cache.free;
            (*cache.free).prev = cache.free;
        }

        // Initialize the memory.
        (0..max_count.get() as isize).for_each(|index| unsafe {
            let node_pointer = cache.pointer.as_ptr().offset(index);
            node_pointer.write(Entry::new_zeroed());
            cache.attach_free(node_pointer);
        });

        cache
    }

    #[inline]
    pub(crate) fn touch(&mut self, key: I) {
        if let Some(node_pointer) = self.entries.get_mut(key).copied() {
            self.detach(node_pointer);
            self.attach(node_pointer);
        }
    }

    pub(crate) fn put(&mut self, key: I, value: V, size: usize) -> bool {
        match self.entries.get_mut(key) {
            Some(node_pointer) => {
                let node_pointer = *node_pointer;

                self.detach(node_pointer);
                self.attach(node_pointer);

                let node = unsafe { &mut *node_pointer };

                node.value = Some(value);
                if node.size != size {
                    self.size -= node.size;
                    self.size += size;
                    node.size = size;
                }

                true
            }
            None => {
                let next_free = unsafe { (*self.free).prev };
                if next_free != self.free {
                    self.size += size;
                    self.count += 1;

                    self.detach(next_free);
                    self.attach(next_free);

                    self.entries.insert(key, next_free);

                    let node = unsafe { &mut *next_free };
                    node.key = Some(key);
                    node.value = Some(value);
                    node.size = size;

                    true
                } else {
                    false
                }
            }
        }
    }

    #[inline(always)]
    pub(crate) fn size(&self) -> usize {
        self.size
    }

    #[inline(always)]
    pub(crate) fn count(&self) -> u32 {
        self.count
    }

    pub(crate) fn remove(&mut self, key: I) -> Option<V> {
        let node_pointer = self.entries.remove(key)?;
        self.detach(node_pointer);
        self.attach_free(node_pointer);

        self.count -= 1;
        unsafe {
            self.size -= (*node_pointer).size;
            (*node_pointer).value.take()
        }
    }

    pub(crate) fn pop(&mut self) -> Option<(I, V)> {
        let prev = unsafe { (*self.list).prev };

        (prev != self.list)
            .then(|| unsafe { (*(*self.list).prev).key })
            .flatten()
            .and_then(|key| self.remove(key).map(|value| (key, value)))
    }

    #[inline]
    fn detach(&mut self, node: *mut Entry<I, V>) {
        unsafe {
            (*(*node).prev).next = (*node).next;
            (*(*node).next).prev = (*node).prev;
        }
    }

    #[inline]
    fn attach(&mut self, node: *mut Entry<I, V>) {
        unsafe {
            (*node).next = (*self.list).next;
            (*node).prev = self.list;
            (*self.list).next = node;
            (*(*node).next).prev = node;
        }
    }

    #[inline]
    fn attach_free(&mut self, node: *mut Entry<I, V>) {
        unsafe {
            (*node).next = (*self.free).next;
            (*node).prev = self.free;
            (*self.free).next = node;
            (*(*node).next).prev = node;
        }
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use super::Lru;
    use crate::container::GenerationalSlab;

    create_generational_key!(TestKey);

    #[test]
    fn test_put() {
        let mut slab = GenerationalSlab::default();
        let key_0 = slab.insert(false).unwrap();
        let key_1 = slab.insert(false).unwrap();

        let mut cache: Lru<TestKey, usize> = Lru::new(NonZeroU32::new(2).unwrap());
        cache.put(key_0, 1, 10);
        cache.put(key_1, 2, 20);
        assert_eq!(cache.size(), 30);
        assert_eq!(cache.count(), 2);
    }

    #[test]
    fn test_put_updates() {
        let mut slab = GenerationalSlab::default();
        let key_0 = slab.insert(false).unwrap();

        let mut cache: Lru<TestKey, usize> = Lru::new(NonZeroU32::new(2).unwrap());
        cache.put(key_0, 1, 10);
        cache.touch(key_0);
        assert_eq!(cache.size(), 10);
        assert_eq!(cache.count(), 1);

        cache.put(key_0, 2, 20);
        assert_eq!(cache.pop(), Some((key_0, 2)));
        assert_eq!(cache.size(), 0);
        assert_eq!(cache.count(), 0);
    }

    #[test]
    fn test_remove() {
        let mut slab = GenerationalSlab::default();
        let key_0 = slab.insert(false).unwrap();
        let key_1 = slab.insert(false).unwrap();
        let key_2 = slab.insert(false).unwrap();

        let mut cache: Lru<TestKey, usize> = Lru::new(NonZeroU32::new(3).unwrap());
        cache.put(key_0, 1, 10);
        cache.put(key_1, 2, 20);
        cache.put(key_2, 3, 30);

        assert_eq!(cache.size(), 60);
        assert_eq!(cache.count(), 3);

        cache.remove(key_1);
        assert_eq!(cache.size(), 40);
        assert_eq!(cache.count(), 2);

        cache.remove(key_2);
        assert_eq!(cache.size(), 10);
        assert_eq!(cache.count(), 1);
    }

    #[test]
    fn test_reuse() {
        let mut slab = GenerationalSlab::default();
        let key_0 = slab.insert(false).unwrap();

        let mut cache: Lru<TestKey, usize> = Lru::new(NonZeroU32::new(2).unwrap());
        cache.put(key_0, 1, 10);
        cache.remove(key_0);
        cache.put(key_0, 1, 20);

        assert_eq!(cache.size(), 20);
        assert_eq!(cache.count(), 1);
    }

    #[test]
    fn test_pop() {
        let mut slab = GenerationalSlab::default();
        let key_0 = slab.insert(false).unwrap();
        let key_1 = slab.insert(false).unwrap();

        let mut cache: Lru<TestKey, usize> = Lru::new(NonZeroU32::new(2).unwrap());
        cache.put(key_0, 1, 10);
        cache.put(key_1, 2, 20);

        assert_eq!(cache.size(), 30);
        assert_eq!(cache.count(), 2);

        assert_eq!(cache.pop(), Some((key_0, 1)));
        assert_eq!(cache.size(), 20);
        assert_eq!(cache.count(), 1);

        assert_eq!(cache.pop(), Some((key_1, 2)));
        assert_eq!(cache.size(), 0);
        assert_eq!(cache.count(), 0);
    }

    #[test]
    fn test_random_remove() {
        use rand::prelude::*;

        let mut slab = GenerationalSlab::default();

        let mut cache: Lru<TestKey, usize> = Lru::new(NonZeroU32::new(1_000).unwrap());

        let mut keys: Vec<TestKey> = (0..1_000)
            .map(|index| {
                let key = slab.insert(false).unwrap();
                cache.put(key, index, index);
                key
            })
            .collect();

        let mut rng = StdRng::from_entropy();
        keys.shuffle(&mut rng);

        keys.drain(..).for_each(|key| {
            cache.remove(key);
        });

        assert_eq!(cache.size(), 0);
        assert_eq!(cache.count(), 0);
    }

    #[test]
    fn test_capacity_reached() {
        let mut slab = GenerationalSlab::default();
        let key_0 = slab.insert(false).unwrap();
        let key_1 = slab.insert(false).unwrap();
        let key_2 = slab.insert(false).unwrap();

        let mut cache: Lru<TestKey, usize> = Lru::new(NonZeroU32::new(2).unwrap());
        assert!(cache.put(key_0, 1, 10));
        assert!(cache.put(key_1, 2, 10));
        assert!(!cache.put(key_2, 3, 10));
    }
}
