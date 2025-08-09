use std::any::Any;
use std::cell::UnsafeCell;
use std::collections::HashMap;

use super::id::{ElementId, ElementIdGenerator};

pub(crate) struct InternalElementStore {
    // TODO: Let the element choose between HashMap and Vec lookup.
    /// The inner element stores need to be in a `Box` so that we can safely
    /// pass out references to them without worrying about
    /// relocation of the hashmap when inserting new children.
    children: HashMap<u64, Box<InternalElementStore>>,
    data: UnsafeCell<Option<Box<dyn Any>>>,
    element_id: ElementId,
}

impl InternalElementStore {
    pub fn root(generator: &mut ElementIdGenerator) -> Self {
        Self {
            children: HashMap::new(),
            data: UnsafeCell::new(None),
            element_id: generator.generate(),
        }
    }
}

/// An immuatble instance of the element store. See [`ElementStoreMut`] for
/// a mutable version.
///
/// This type doesn't implement Clone or Copy by design. Calling
/// [`lay_out`](crate::element::Element::lay_out) will
/// consume the store, making the ownership system catch cases where the
/// same store is accidentally passed to multiple elements.
pub struct ElementStore<'a> {
    element_store: &'a InternalElementStore,
    window_id: u64,
}

impl<'a> ElementStore<'a> {
    pub(crate) fn new(element_store: &'a InternalElementStore, window_id: u64) -> Self {
        Self { element_store, window_id }
    }

    pub fn get_window_id(&self) -> u64 {
        self.window_id
    }

    pub fn child_store(&self, index: u64) -> Self {
        let element_store = self.element_store.children.get(&index).expect("Tried to get invalid child store");

        ElementStore {
            element_store,
            window_id: self.window_id,
        }
    }

    pub fn get_element_id(&self) -> ElementId {
        self.element_store.element_id
    }
}

/// A muatble instance of the element store. See [`ElementStore`] for an
/// immutable version.
///
/// Calling
/// [`create_layout_info`](crate::element::Element::create_layout_info) will
/// consume the store, making the ownership system catch cases where the
/// same store is accidentally passed to multiple elements.
pub struct ElementStoreMut<'a> {
    element_store: &'a mut InternalElementStore,
    generator: &'a mut ElementIdGenerator,
    window_id: u64,
}

impl<'a> ElementStoreMut<'a> {
    pub(crate) fn new(element_store: &'a mut InternalElementStore, generator: &'a mut ElementIdGenerator, window_id: u64) -> Self {
        Self {
            element_store,
            generator,
            window_id,
        }
    }

    pub fn get_window_id(&self) -> u64 {
        self.window_id
    }

    pub fn child_store(&mut self, index: u64) -> ElementStoreMut<'_> {
        let element_store = self
            .element_store
            .children
            .entry(index)
            .or_insert_with(|| Box::new(InternalElementStore::root(self.generator)));

        ElementStoreMut {
            element_store,
            generator: self.generator,
            window_id: self.window_id,
        }
    }

    pub fn get_element_id(&self) -> ElementId {
        self.element_store.element_id
    }
}

pub trait PersistentDataProvider<'a> {
    fn get_data<T>(&self, inputs: T::Inputs) -> &'a T
    where
        T: Any + PersistentData;
}

impl<'a> PersistentDataProvider<'a> for ElementStore<'a> {
    fn get_data<T>(&self, inputs: T::Inputs) -> &'a T
    where
        T: Any + PersistentData,
    {
        let data = unsafe { &mut *self.element_store.data.get() };

        data.get_or_insert_with(|| Box::new(T::from_inputs(inputs))).downcast_ref().unwrap()
    }
}

impl<'a> PersistentDataProvider<'a> for ElementStoreMut<'a> {
    fn get_data<T>(&self, inputs: T::Inputs) -> &'a T
    where
        T: Any + PersistentData,
    {
        let data = unsafe { &mut *self.element_store.data.get() };

        data.get_or_insert_with(|| Box::new(T::from_inputs(inputs))).downcast_ref().unwrap()
    }
}

pub trait PersistentData: 'static {
    type Inputs;

    fn from_inputs(inputs: Self::Inputs) -> Self;
}

impl<T> PersistentData for T
where
    T: Default + 'static,
{
    type Inputs = ();

    fn from_inputs(_: Self::Inputs) -> Self {
        Self::default()
    }
}

pub trait Persistent {
    type Data: PersistentData;
}

pub trait PersistentExt: Persistent {
    fn get_persistent_data<'a>(
        &self,
        store: &impl PersistentDataProvider<'a>,
        inputs: <Self::Data as PersistentData>::Inputs,
    ) -> &'a Self::Data;
}

impl<T> PersistentExt for T
where
    T: Persistent,
{
    fn get_persistent_data<'a>(
        &self,
        store: &impl PersistentDataProvider<'a>,
        inputs: <Self::Data as PersistentData>::Inputs,
    ) -> &'a Self::Data {
        store.get_data::<Self::Data>(inputs)
    }
}
