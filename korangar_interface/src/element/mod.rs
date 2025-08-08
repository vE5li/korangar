mod state;

use std::marker::PhantomData;

pub use interface_macros::StateElement;
use rust_state::Context;
use store::{ElementStore, ElementStoreMut};

pub use self::state::*;
use crate::application::Application;
use crate::layout::area::Area;
use crate::layout::{Layout, Resolver};

pub mod id {
    use std::any::{Any, TypeId};

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub struct ElementId(usize);

    #[derive(Clone)]
    pub(crate) struct ElementIdGenerator {
        next_free_id: usize,
    }

    impl ElementIdGenerator {
        pub fn new() -> Self {
            Self { next_free_id: 0 }
        }

        pub fn generate(&mut self) -> ElementId {
            let id = ElementId(self.next_free_id);

            self.next_free_id += 1;

            id
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub struct FocusId(TypeId);

    pub trait FocusIdExt {
        fn focus_id(&self) -> FocusId;
    }

    impl<T> FocusIdExt for T
    where
        T: Any,
    {
        fn focus_id(&self) -> FocusId {
            FocusId(self.type_id())
        }
    }
}

pub mod store {
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

            data.get_or_insert_with(|| Box::new(T::new(inputs))).downcast_ref().unwrap()
        }
    }

    impl<'a> PersistentDataProvider<'a> for ElementStoreMut<'a> {
        fn get_data<T>(&self, inputs: T::Inputs) -> &'a T
        where
            T: Any + PersistentData,
        {
            let data = unsafe { &mut *self.element_store.data.get() };

            data.get_or_insert_with(|| Box::new(T::new(inputs))).downcast_ref().unwrap()
        }
    }

    pub trait PersistentData: 'static {
        type Inputs;

        fn new(inputs: Self::Inputs) -> Self;
    }

    impl<T> PersistentData for T
    where
        T: Default + 'static,
    {
        type Inputs = ();

        fn new(_: Self::Inputs) -> Self {
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
}

pub struct BaseLayoutInfo {
    pub area: Area,
}

pub struct DefaultLayoutInfo<App: Application> {
    pub area: Area,
    pub font_size: App::FontSize,
}

pub struct DefaultLayoutInfoSet<T> {
    pub area: Area,
    pub children: T,
}

pub trait Element<App: Application> {
    type LayoutInfo = DefaultLayoutInfo<App>;

    fn create_layout_info(
        &mut self,
        state: &Context<App>,
        store: ElementStoreMut<'_>,
        resolver: &mut Resolver<'_, App>,
    ) -> Self::LayoutInfo;

    fn lay_out<'a>(
        &'a self,
        state: &'a Context<App>,
        store: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut Layout<'a, App>,
    );
}

pub trait ResolverSet<'a, App: Application> {
    fn with_index<C>(&mut self, index: usize, f: impl FnMut(&mut Resolver<'a, App>) -> C) -> C;
}

impl<'a, App> ResolverSet<'a, App> for &mut Resolver<'a, App>
where
    App: Application,
{
    fn with_index<C>(&mut self, _: usize, mut f: impl FnMut(&mut Resolver<'a, App>) -> C) -> C {
        f(*self)
    }
}

pub trait ElementSet<App: Application> {
    type LayoutInfo;

    fn get_element_count(&self, state: &Context<App>) -> usize;

    fn create_layout_info(
        &mut self,
        state: &Context<App>,
        store: ElementStoreMut<'_>,
        resolver_set: impl ResolverSet<'_, App>,
    ) -> Self::LayoutInfo;

    fn lay_out<'a>(
        &'a self,
        state: &'a Context<App>,
        store: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut Layout<'a, App>,
    );
}

impl<App> ElementSet<App> for ()
where
    App: Application,
{
    type LayoutInfo = ();

    fn get_element_count(&self, _: &Context<App>) -> usize {
        0
    }

    fn create_layout_info(&mut self, _: &Context<App>, _: ElementStoreMut<'_>, _: impl ResolverSet<'_, App>) {}

    fn lay_out<'a>(&'a self, _: &'a Context<App>, _: ElementStore<'a>, _: &'a Self::LayoutInfo, _: &mut Layout<'a, App>) {}
}

impl<App, T, const N: usize> ElementSet<App> for [T; N]
where
    App: Application,
    T: Element<App>,
{
    type LayoutInfo = [T::LayoutInfo; N];

    fn get_element_count(&self, _: &Context<App>) -> usize {
        N
    }

    fn create_layout_info(
        &mut self,
        state: &Context<App>,
        mut store: ElementStoreMut<'_>,
        mut resolver_set: impl ResolverSet<'_, App>,
    ) -> Self::LayoutInfo {
        std::array::from_fn(|index| {
            resolver_set.with_index(index, |resolver| {
                self[index].create_layout_info(state, store.child_store(index as u64), resolver)
            })
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a Context<App>,
        store: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut Layout<'a, App>,
    ) {
        for index in 0..N {
            self[index].lay_out(state, store.child_store(index as u64), &layout_info[index], layout);
        }
    }
}

#[crabtime::function]
fn impl_element_set(up_to: usize) {
    for number_of_elements in 1..=up_to {
        let elements = (0..number_of_elements).map(|index| format!("T{index}"));
        let element_list: String = elements.clone().map(|element| format!("{element}, ")).collect();
        let layout_info: String = elements.clone().map(|element| format!("{element}::LayoutInfo, ")).collect();
        let bounds: String = elements.map(|element| format!("{element}: Element<App>, ")).collect();

        let create_layout_infos: String = (0..number_of_elements)
            .map(|index| {
                format!(
                    "
                    resolver_set.with_index({index}, |resolver| {{
                        self.{index}.create_layout_info(state, store.child_store({index}), resolver)
                    }}), "
                )
            })
            .collect();

        let lay_outs: String = (0..number_of_elements)
            .map(|index| format!("self.{index}.lay_out(state, store.child_store({index}), &layout_info.{index}, layout);"))
            .collect();

        crabtime::output! {
            impl<App, {{element_list}}> ElementSet<App> for ({{element_list}})
            where
                App: Application,
                {{bounds}}
            {
                type LayoutInfo = ({{layout_info}});

                fn get_element_count(&self, _: &Context<App>) -> usize {
                    {{number_of_elements}}
                }

                fn create_layout_info(
                    &mut self,
                    state: &Context<App>,
                    mut store: ElementStoreMut<'_>,
                    mut resolver_set: impl ResolverSet<'_, App>,
                ) -> Self::LayoutInfo {
                    ({{create_layout_infos}})
                }

                fn lay_out<'a>(
                    &'a self,
                    state: &'a Context<App>,
                    store: ElementStore<'a>,
                    layout_info: &'a Self::LayoutInfo,
                    layout: &mut Layout<'a, App>,
                ) {
                    {{lay_outs}}
                }
            }
        }
    }
}

// Implement `ElementSet` for tuples up to 64 elements.
impl_element_set!(64);

pub struct ErasedElement<App, E>
where
    App: Application,
    E: Element<App>,
{
    element: E,
    layout_info: Option<E::LayoutInfo>,
    _marker: PhantomData<App>,
}

impl<App, E> ErasedElement<App, E>
where
    App: Application,
    E: Element<App>,
{
    pub fn new(element: E) -> Box<Self> {
        Box::new(Self {
            element,
            layout_info: None,
            _marker: PhantomData,
        })
    }
}

impl<App, E> Element<App> for ErasedElement<App, E>
where
    App: Application,
    E: Element<App>,
    E::LayoutInfo: 'static,
{
    type LayoutInfo = ();

    fn create_layout_info(&mut self, state: &Context<App>, store: ElementStoreMut<'_>, resolver: &mut Resolver<'_, App>) {
        let layout_info = self.element.create_layout_info(state, store, resolver);
        self.layout_info = Some(layout_info);
    }

    fn lay_out<'a>(&'a self, state: &'a Context<App>, store: ElementStore<'a>, _: &'a Self::LayoutInfo, layout: &mut Layout<'a, App>) {
        let layout_info = self.layout_info.as_ref().expect("no layout created");
        self.element.lay_out(state, store, layout_info, layout);
    }
}

pub type ElementBox<App> = Box<dyn Element<App, LayoutInfo = ()>>;
