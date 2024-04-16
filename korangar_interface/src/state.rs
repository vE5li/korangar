use std::cell::{Ref, RefCell};
use std::marker::PhantomData;
use std::ops::Not;
use std::rc::Rc;

use super::ClickAction;
use crate::application::Application;

/// The state of a value borrowed by [`borrow_mut`](TrackedState::with_mut).
pub enum ValueState<T> {
    Mutated(T),
    Unchanged(T),
}

/// The version of the current value inside a [`TrackedState`]. This is used to
/// update [`Remote`]s when the value changes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(usize);

impl Version {
    /// Get a [`Version`] from a [`usize`].
    pub fn from_raw(version: usize) -> Self {
        Self(version)
    }

    /// Bump the version by 1.
    fn bump(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }
}

/// Shared state that keeps track of mutations to the inner value. It can create
/// [`Remote`]s to listen to state changes.
pub trait TrackedState<Value>: Clone {
    type RemoteType: Remote<Value> + 'static;

    /// Set the inner value.
    fn set(&mut self, value: Value);

    /// Get an immutable reference to the inner value.
    fn get(&self) -> Ref<'_, Value>;

    /// Get the current [`Version`].
    fn get_version(&self) -> Version;

    /// Work on a mutable reference of the inner value. The provided closure has
    /// to report back whether or not the value inside was mutated. If any state
    /// change occurred, the closure *must* return [`ValueState::Mutated`].
    ///
    /// NOTE: It is imperative that the correct state is
    /// returned. Returning [`ValueState::Mutated`] when no modification was
    /// done might result in unnecessary updates. Similarly, returning
    /// [`ValueState::Unchanged`] when the value was mutated will result in
    /// no update at all.
    fn with_mut<Closure, Return>(&mut self, closure: Closure) -> Return
    where
        Closure: FnOnce(&mut Value) -> ValueState<Return>;

    /// Advance the version of the state without modifying the inner value.
    fn update(&mut self);

    /// Create a new [`Remote`].
    fn new_remote(&self) -> Self::RemoteType;
}

/// Extension trait to mutate the inner value of a [`TrackedState`].
pub trait TrackedStateExt<Value> {
    fn mutate<Closure, Return>(&mut self, closure: Closure) -> Return
    where
        Closure: FnOnce(&mut Value) -> Return;
}

impl<T, Value> TrackedStateExt<Value> for T
where
    T: TrackedState<Value>,
{
    fn mutate<Closure, Return>(&mut self, closure: Closure) -> Return
    where
        Closure: FnOnce(&mut Value) -> Return,
    {
        self.with_mut(|value| ValueState::Mutated(closure(value)))
    }
}

#[derive(Default)]
struct InnerState<Value> {
    value: Value,
    version: Version,
}

impl<Value> InnerState<Value> {
    pub fn new(value: Value) -> Self {
        Self {
            value,
            version: Version::default(),
        }
    }
}

#[derive(Default)]
pub struct PlainTrackedState<Value>(Rc<RefCell<InnerState<Value>>>);

impl<Value> PlainTrackedState<Value> {
    pub fn new(value: Value) -> PlainTrackedState<Value> {
        Self(Rc::new(RefCell::new(InnerState::new(value))))
    }

    pub fn mapped<As, F>(&self, mapping: F) -> MappedTrackedState<Value, As, F>
    where
        F: Fn(&Value) -> &As,
    {
        MappedTrackedState {
            state: self.clone(),
            mapping,
            marker: PhantomData,
        }
    }

    pub fn mapped_remote<As, F>(&self, mapping: F) -> MappedRemote<Value, As, F>
    where
        F: Fn(&Value) -> &As,
    {
        let tracked_state = self.mapped(mapping);

        MappedRemote {
            tracked_state,
            version: self.get_version(),
        }
    }

    pub fn foo_test(&self) {
        println!("Weak count: {}", Rc::weak_count(&self.0));
        println!("Strong count: {}", Rc::strong_count(&self.0));
    }
}

impl<Value> Clone for PlainTrackedState<Value> {
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}

impl<Value> TrackedState<Value> for PlainTrackedState<Value>
where
    Value: 'static,
{
    type RemoteType = PlainRemote<Value>;

    fn set(&mut self, value: Value) {
        let mut inner = self.0.borrow_mut();
        inner.value = value;
        inner.version.bump();
    }

    fn get(&self) -> Ref<'_, Value> {
        Ref::map(self.0.borrow(), |inner| &inner.value)
    }

    fn get_version(&self) -> Version {
        self.0.borrow().version
    }

    fn with_mut<Closure, Return>(&mut self, closure: Closure) -> Return
    where
        Closure: FnOnce(&mut Value) -> ValueState<Return>,
    {
        let mut inner = self.0.borrow_mut();

        match closure(&mut inner.value) {
            ValueState::Mutated(return_value) => {
                inner.version.bump();
                return_value
            }
            ValueState::Unchanged(return_value) => return_value,
        }
    }

    fn update(&mut self) {
        self.0.borrow_mut().version.bump();
    }

    fn new_remote(&self) -> Self::RemoteType {
        let tracked_state = self.clone();
        let version = self.get_version();

        PlainRemote { tracked_state, version }
    }
}

pub struct MappedTrackedState<Value, As, F>
where
    Value: 'static,
    As: 'static,
    F: Fn(&Value) -> &As + 'static,
{
    state: PlainTrackedState<Value>,
    mapping: F,
    marker: PhantomData<As>,
}

impl<Value, As, F> Clone for MappedTrackedState<Value, As, F>
where
    F: Clone + Fn(&Value) -> &As,
{
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            mapping: self.mapping.clone(),
            marker: PhantomData,
        }
    }
}

impl<Value, As, F> TrackedState<As> for MappedTrackedState<Value, As, F>
where
    Value: 'static,
    As: 'static,
    F: Clone + Fn(&Value) -> &As + 'static,
{
    type RemoteType = MappedRemote<Value, As, F>;

    fn set(&mut self, value: As) {
        let mut inner = self.state.0.borrow_mut();
        let mapped = (self.mapping)(&inner.value);
        // SAFETY: This operation _should_ be perfectly safe, since we just casted from
        // the mutable reference to an immutable one. This bit of unsafety makes
        // the creation of mappings much easier.
        #[allow(mutable_transmutes)]
        let mapped = unsafe { std::mem::transmute::<&As, &mut As>(mapped) };

        *mapped = value;
        inner.version.bump();
    }

    fn get(&self) -> Ref<'_, As> {
        Ref::map(self.state.get(), &self.mapping)
    }

    fn get_version(&self) -> Version {
        self.state.get_version()
    }

    fn with_mut<Closure, Return>(&mut self, closure: Closure) -> Return
    where
        Closure: FnOnce(&mut As) -> ValueState<Return>,
    {
        let mut inner = self.state.0.borrow_mut();
        let mapped = (self.mapping)(&inner.value);
        // SAFETY: This operation _should_ be perfectly safe, since we just casted from
        // the mutable reference to an immutable one. This bit of unsafety makes
        // the creation of mappings much easier.
        #[allow(mutable_transmutes)]
        let mapped = unsafe { std::mem::transmute::<&As, &mut As>(mapped) };

        match closure(mapped) {
            ValueState::Mutated(return_value) => {
                inner.version.bump();
                return_value
            }
            ValueState::Unchanged(return_value) => return_value,
        }
    }

    fn update(&mut self) {
        self.state.update();
    }

    fn new_remote(&self) -> Self::RemoteType {
        MappedRemote {
            tracked_state: self.clone(),
            version: self.state.get_version(),
        }
    }
}

pub trait TrackedStateClone<Value>: TrackedState<Value> {
    fn cloned(&self) -> Value;
}

impl<T, Value> TrackedStateClone<Value> for T
where
    T: TrackedState<Value>,
    Value: Clone,
{
    fn cloned(&self) -> Value {
        self.get().clone()
    }
}

pub trait TrackedStateTake<Value>: TrackedState<Value> {
    fn take(&mut self) -> Value;
}

impl<T, Value> TrackedStateTake<Value> for T
where
    T: TrackedState<Value>,
    Value: Default,
{
    fn take(&mut self) -> Value {
        let mut taken_value = Value::default();

        self.with_mut(|value| {
            std::mem::swap(&mut taken_value, value);
            ValueState::Mutated(())
        });

        taken_value
    }
}

pub trait TrackedStateVec<Value> {
    fn clear(&mut self);

    fn push(&mut self, item: Value);

    fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&Value) -> bool;

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool;
}

impl<T, Value> TrackedStateVec<Value> for T
where
    T: TrackedState<Vec<Value>>,
{
    fn clear(&mut self) {
        self.with_mut(|value| {
            value.clear();
            ValueState::Mutated(())
        });
    }

    fn push(&mut self, item: Value) {
        self.with_mut(|value| {
            value.push(item);
            ValueState::Mutated(())
        });
    }

    fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&Value) -> bool,
    {
        self.with_mut(|value| {
            let previous_length = value.len();

            value.retain_mut(|element| f(element));

            let new_length = value.len();

            match new_length < previous_length {
                true => ValueState::Mutated(()),
                false => ValueState::Unchanged(()),
            }
        });
    }

    fn len(&self) -> usize {
        self.get().len()
    }

    fn is_empty(&self) -> bool {
        self.get().is_empty()
    }
}

pub trait TrackedStateBinary<Value>: TrackedState<Value> {
    fn toggle(&mut self);

    fn selector(&self) -> impl Fn() -> Value + 'static;

    fn toggle_action<App>(&self) -> impl FnMut() -> Vec<ClickAction<App>> + 'static
    where
        App: Application;
}

impl<T, Value> TrackedStateBinary<Value> for T
where
    T: TrackedState<Value> + 'static + Clone, // TODO: Move this clone bound elsewhere
    Value: Not<Output = Value> + Copy,
{
    fn toggle(&mut self) {
        self.with_mut(|value| {
            *value = !*value;
            ValueState::Mutated(())
        });
    }

    fn selector(&self) -> impl Fn() -> Value + 'static {
        let cloned = self.clone();
        move || cloned.cloned()
    }

    fn toggle_action<App>(&self) -> impl FnMut() -> Vec<ClickAction<App>> + 'static
    where
        App: Application,
    {
        let mut cloned = (*self).clone();
        move || {
            cloned.toggle();
            Vec::new()
        }
    }
}

pub trait Remote<Value> {
    type State;

    fn clone_state(&self) -> Self::State;

    fn get(&self) -> Ref<'_, Value>;

    fn consume_changed(&mut self) -> bool;
}

pub trait RemoteClone<Value>: Remote<Value> {
    fn cloned(&self) -> Value;
}

impl<T, Value> RemoteClone<Value> for T
where
    T: Remote<Value>,
    Value: Clone,
{
    fn cloned(&self) -> Value {
        self.get().clone()
    }
}

pub struct PlainRemote<Value> {
    tracked_state: PlainTrackedState<Value>,
    version: Version,
}

impl<Value> PlainRemote<Value>
where
    Value: 'static,
{
    pub fn new(value: Value) -> PlainRemote<Value> {
        PlainTrackedState::new(value).new_remote()
    }
}

impl<Value> Remote<Value> for PlainRemote<Value>
where
    Value: 'static,
{
    type State = PlainTrackedState<Value>;

    fn clone_state(&self) -> PlainTrackedState<Value> {
        self.tracked_state.clone()
    }

    fn get(&self) -> Ref<'_, Value> {
        self.tracked_state.get()
    }

    fn consume_changed(&mut self) -> bool {
        let version = self.tracked_state.get_version();
        let changed = self.version != version;
        self.version = version;

        changed
    }
}

impl<Value> Clone for PlainRemote<Value> {
    fn clone(&self) -> Self {
        let tracked_state = self.tracked_state.clone();
        let version = self.version;

        Self { tracked_state, version }
    }
}

pub struct MappedRemote<Value, As, F>
where
    Value: 'static,
    As: 'static,
    F: Fn(&Value) -> &As + 'static,
{
    tracked_state: MappedTrackedState<Value, As, F>,
    version: Version,
}

impl<Value, As, F> Remote<As> for MappedRemote<Value, As, F>
where
    Value: 'static,
    As: 'static,
    F: Clone + Fn(&Value) -> &As + 'static,
{
    type State = MappedTrackedState<Value, As, F>;

    fn clone_state(&self) -> Self::State {
        self.tracked_state.clone()
    }

    fn get(&self) -> Ref<'_, As> {
        self.tracked_state.get()
    }

    fn consume_changed(&mut self) -> bool {
        let version = self.tracked_state.get_version();
        let changed = self.version != version;
        self.version = version;

        changed
    }
}

impl<Value, As, F> Clone for MappedRemote<Value, As, F>
where
    F: Clone + Fn(&Value) -> &As,
{
    fn clone(&self) -> Self {
        let tracked_state = self.tracked_state.clone();
        let version = self.version;

        Self { tracked_state, version }
    }
}
