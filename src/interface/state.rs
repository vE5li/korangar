use std::cell::{Ref, RefCell};
use std::ops::Not;
use std::rc::Rc;

use super::StateProvider;

#[derive(Default)]
pub struct TrackedState<T>(Rc<RefCell<(T, usize)>>);

impl<T> TrackedState<T> {
    pub fn new(value: T) -> TrackedState<T> {
        Self(Rc::new(RefCell::new((value, 0))))
    }

    pub fn set(&mut self, data: T) {
        let mut inner = self.0.borrow_mut();
        inner.0 = data;
        inner.1 = inner.1.wrapping_add(1);
    }

    pub fn borrow(&self) -> Ref<'_, T> {
        Ref::map(self.0.borrow(), |inner| &inner.0)
    }

    pub fn get_version(&self) -> usize {
        self.0.borrow().1
    }

    pub fn with_mut<F>(&mut self, f: F)
    where
        F: FnOnce(&mut T, &mut dyn FnMut()),
    {
        let (inner, version) = &mut *self.0.borrow_mut();
        let mut changed = || *version = version.wrapping_add(1);

        f(inner, &mut changed)
    }

    pub fn update(&mut self) {
        let mut inner = self.0.borrow_mut();
        inner.1 = inner.1.wrapping_add(1);
    }

    pub fn new_remote(&self) -> Remote<T> {
        let tracked_state = self.clone();
        let version = self.get_version();

        Remote { tracked_state, version }
    }
}

impl<T> TrackedState<Vec<T>> {
    pub fn clear(&mut self) {
        let mut inner = self.0.borrow_mut();
        inner.0.clear();
        inner.1 = inner.1.wrapping_add(1);
    }

    pub fn push(&mut self, item: T) {
        let mut inner = self.0.borrow_mut();
        inner.0.push(item);
        inner.1 = inner.1.wrapping_add(1);
    }

    pub fn append(&mut self, other: &mut Vec<T>) {
        let mut inner = self.0.borrow_mut();
        inner.0.append(other);
        inner.1 = inner.1.wrapping_add(1);
    }

    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&T) -> bool,
    {
        let mut inner = self.0.borrow_mut();
        let previous_length = inner.0.len();

        inner.0.retain_mut(|element| f(element));

        let new_length = inner.0.len();
        let has_updated = new_length < previous_length;

        // Litte hack to save a branch.
        inner.1 = inner.1.wrapping_add(has_updated as usize);
    }

    pub fn len(&self) -> usize {
        self.borrow().len()
    }
}

impl<T> TrackedState<Option<T>>
where
    T: Default,
{
    pub fn take(&mut self) -> Option<T> {
        self.0.borrow_mut().0.take()
    }
}

impl<T> TrackedState<T>
where
    T: Not<Output = T> + Copy,
{
    pub fn toggle(&mut self) {
        let mut inner = self.0.borrow_mut();
        inner.0 = !inner.0;
        inner.1 = inner.1.wrapping_add(1);
    }

    pub fn toggle_action(&self) -> impl FnMut() {
        let mut cloned = self.clone();
        move || cloned.toggle()
    }
}

impl TrackedState<bool> {
    pub fn selector(&self) -> impl Fn(&StateProvider) -> bool {
        let cloned = self.clone();
        move |_: &StateProvider| *cloned.borrow()
    }
}

impl<T> Clone for TrackedState<T> {
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}

pub struct Remote<T> {
    tracked_state: TrackedState<T>,
    version: usize,
}

impl<T> Remote<T> {
    pub fn borrow(&self) -> Ref<'_, T> {
        self.tracked_state.borrow()
    }

    pub fn consume_changed(&mut self) -> bool {
        let version = self.tracked_state.get_version();
        let changed = self.version != version;
        self.version = version;

        changed
    }
}

impl<T> Clone for Remote<T> {
    fn clone(&self) -> Self {
        let tracked_state = self.tracked_state.clone();
        let version = self.version;

        Self { tracked_state, version }
    }
}
