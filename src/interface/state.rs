use std::cell::{Ref, RefCell};
use std::ops::Not;
use std::rc::Rc;

use super::{ClickAction, StateProvider};

/// The state of a value borrowed by [`borrow_mut`](TrackedState::with_mut).
pub enum ValueState<T> {
    Mutated(T),
    Unchanged(T),
}

#[derive(Default)]
struct InnerState<VALUE> {
    value: VALUE,
    version: usize,
}

impl<VALUE> InnerState<VALUE> {
    pub fn new(value: VALUE) -> Self {
        Self { value, version: 0 }
    }

    pub fn bump_version(&mut self) {
        self.version = self.version.wrapping_add(1);
    }
}

#[derive(Default)]
pub struct TrackedState<VALUE>(Rc<RefCell<InnerState<VALUE>>>);

impl<VALUE> TrackedState<VALUE> {
    pub fn new(value: VALUE) -> TrackedState<VALUE> {
        Self(Rc::new(RefCell::new(InnerState::new(value))))
    }

    pub fn set(&mut self, data: VALUE) {
        let mut inner = self.0.borrow_mut();
        inner.value = data;
        inner.bump_version();
    }

    pub fn borrow(&self) -> Ref<'_, VALUE> {
        Ref::map(self.0.borrow(), |inner| &inner.value)
    }

    pub fn get_version(&self) -> usize {
        self.0.borrow().version
    }

    /// Work on a mutable reference of the inner value. The provided closure has
    /// to report back whether or not the value inside was mutated. If any state
    /// change occurred, the closure *must* return [`ValueState::Mutated`].
    ///
    /// NOTE: It is imperative that the correct state is
    /// returned. Returning [`ValueState::Mutated`] when no modification was
    /// done might result in unnecessary updates. Similarly, returning
    /// [`ValueState::Unchanged`] when the value was mutated will result in
    /// no update at all.
    pub fn with_mut<CLOSURE, RETURN>(&mut self, closure: CLOSURE) -> RETURN
    where
        CLOSURE: FnOnce(&mut VALUE) -> ValueState<RETURN>,
    {
        let mut inner = self.0.borrow_mut();

        match closure(&mut inner.value) {
            ValueState::Mutated(return_value) => {
                inner.bump_version();
                return_value
            }
            ValueState::Unchanged(return_value) => return_value,
        }
    }

    pub fn update(&mut self) {
        self.0.borrow_mut().bump_version();
    }

    pub fn new_remote(&self) -> Remote<VALUE> {
        let tracked_state = self.clone();
        let version = self.get_version();

        Remote { tracked_state, version }
    }
}

impl<VALUE> TrackedState<VALUE>
where
    VALUE: Clone,
{
    pub fn get(&self) -> VALUE {
        self.0.borrow().value.clone()
    }
}

impl<VALUE> TrackedState<Vec<VALUE>> {
    pub fn clear(&mut self) {
        let mut inner = self.0.borrow_mut();
        inner.value.clear();
        inner.bump_version();
    }

    pub fn push(&mut self, item: VALUE) {
        let mut inner = self.0.borrow_mut();
        inner.value.push(item);
        inner.bump_version();
    }

    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&VALUE) -> bool,
    {
        let mut inner = self.0.borrow_mut();
        let previous_length = inner.value.len();

        inner.value.retain_mut(|element| f(element));

        let new_length = inner.value.len();
        let has_updated = new_length < previous_length;

        if has_updated {
            inner.bump_version();
        }
    }

    pub fn len(&self) -> usize {
        self.borrow().len()
    }
}

pub trait TrackedStateTake<VALUE> {
    fn take(&mut self) -> VALUE;
}

impl<VALUE> TrackedStateTake<VALUE> for TrackedState<VALUE>
where
    VALUE: Default,
{
    default fn take(&mut self) -> VALUE {
        let mut taken_value = VALUE::default();
        let inner_value = &mut self.0.borrow_mut().value;
        std::mem::swap(&mut taken_value, inner_value);

        taken_value
    }
}

impl<VALUE> TrackedStateTake<Option<VALUE>> for TrackedState<Option<VALUE>>
where
    VALUE: Default,
{
    fn take(&mut self) -> Option<VALUE> {
        let option = self.0.borrow_mut().value.take();

        // NOTE: Unnecessary updates might have huge impacts on performance, so we try
        // to only update if we actually took a value.
        if option.is_some() {
            self.update();
        }

        option
    }
}

impl<VALUE> TrackedState<VALUE>
where
    VALUE: Not<Output = VALUE> + Copy,
{
    pub fn toggle(&mut self) {
        let mut inner = self.0.borrow_mut();
        inner.value = !inner.value;
        inner.bump_version();
    }

    pub fn toggle_action(&self) -> Box<impl FnMut() -> Vec<ClickAction>> {
        let mut cloned = self.clone();
        Box::new(move || {
            cloned.toggle();
            Vec::new()
        })
    }
}

impl TrackedState<bool> {
    pub fn selector(&self) -> impl Fn(&StateProvider) -> bool {
        let cloned = self.clone();
        move |_: &StateProvider| cloned.get()
    }
}

impl<VALUE> Clone for TrackedState<VALUE> {
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}

pub struct Remote<VALUE> {
    tracked_state: TrackedState<VALUE>,
    version: usize,
}

impl<VALUE> Remote<VALUE> {
    pub fn new(value: VALUE) -> Remote<VALUE> {
        TrackedState::new(value).new_remote()
    }

    pub fn clone_state(&self) -> TrackedState<VALUE> {
        self.tracked_state.clone()
    }

    pub fn borrow(&self) -> Ref<'_, VALUE> {
        self.tracked_state.borrow()
    }

    pub fn consume_changed(&mut self) -> bool {
        let version = self.tracked_state.get_version();
        let changed = self.version != version;
        self.version = version;

        changed
    }
}

impl<VALUE> Remote<VALUE>
where
    VALUE: Clone,
{
    pub fn get(&self) -> VALUE {
        self.tracked_state.get()
    }
}

impl<VALUE> Clone for Remote<VALUE> {
    fn clone(&self) -> Self {
        let tracked_state = self.tracked_state.clone();
        let version = self.version;

        Self { tracked_state, version }
    }
}
