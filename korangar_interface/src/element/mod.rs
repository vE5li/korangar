mod state;

use std::marker::PhantomData;

use id::ElementIdGenerator;
pub use interface_macros::StateElement;
use rust_state::Context;
use store::ElementStore;

pub use self::state::*;
use crate::application::Application;
use crate::layout::area::Area;
use crate::layout::{Layout, Resolver};

pub mod id {
    use std::any::{Any, TypeId};

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub struct ElementId(usize);

    #[derive(Clone)]
    pub struct ElementIdGenerator {
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

    pub struct ElementStore {
        // TODO: Let the element choose between HashMap and Vec lookup.
        /// The inner element stores need to be in a `Box` so that we can safely
        /// pass out references to them without worrying about
        /// relocation of the hashmap when inserting new children.
        children: HashMap<u64, Box<ElementStore>>,
        data: UnsafeCell<Option<Box<dyn Any>>>,
        element_id: ElementId,
    }

    impl ElementStore {
        pub fn root(generator: &mut ElementIdGenerator) -> Self {
            Self {
                children: HashMap::new(),
                data: UnsafeCell::new(None),
                element_id: generator.generate(),
            }
        }

        pub fn get_or_create_child_store(&mut self, index: u64, generator: &mut ElementIdGenerator) -> &mut Self {
            self.children.entry(index).or_insert_with(|| {
                Box::new(ElementStore {
                    // TODO: Maybe deduplicate this code
                    children: HashMap::new(),
                    data: UnsafeCell::new(None),
                    element_id: generator.generate(),
                })
            })
        }

        pub fn child_store(&self, index: u64) -> &Self {
            self.children.get(&index).expect("Tried to get invalid child store")
        }

        fn get_data<T>(&self, inputs: T::Inputs) -> &T
        where
            T: Any + PersistentData,
        {
            let data = unsafe { &mut *self.data.get() };

            data.get_or_insert_with(|| Box::new(T::new(inputs))).downcast_ref().unwrap()
        }

        pub fn get_element_id(&self) -> ElementId {
            self.element_id
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
        fn get_persistent_data<'a>(&self, store: &'a ElementStore, inputs: <Self::Data as PersistentData>::Inputs) -> &'a Self::Data;
    }

    impl<T> PersistentExt for T
    where
        T: Persistent,
    {
        fn get_persistent_data<'a>(&self, store: &'a ElementStore, inputs: <Self::Data as PersistentData>::Inputs) -> &'a Self::Data {
            store.get_data::<Self::Data>(inputs)
        }
    }
}

pub struct DefaultLayoutInfo {
    pub area: Area,
}

pub struct DefaultLayoutInfoSet<T> {
    pub area: Area,
    pub children: T,
}

pub trait Element<App: Application> {
    type LayoutInfo = DefaultLayoutInfo;

    fn create_layout_info(
        &mut self,
        state: &Context<App>,
        store: &mut ElementStore,
        generator: &mut ElementIdGenerator,
        resolver: &mut Resolver,
    ) -> Self::LayoutInfo;

    fn layout_element<'a>(
        &'a self,
        state: &'a Context<App>,
        store: &'a ElementStore,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut Layout<'a, App>,
    );
}

pub trait ResolverSet {
    fn with_index<C>(&mut self, index: usize, f: impl FnMut(&mut Resolver) -> C) -> C;
}

impl ResolverSet for &mut Resolver {
    fn with_index<C>(&mut self, _: usize, mut f: impl FnMut(&mut Resolver) -> C) -> C {
        f(*self)
    }
}

pub trait ElementSet<App: Application> {
    type LayoutInfo;

    fn get_element_count(&self) -> usize;

    fn create_layout_info(
        &mut self,
        state: &Context<App>,
        store: &mut ElementStore,
        // TODO: I think that we don't need this here anymore as of now. It could be bundled with
        // the ElementStore to make an ElementStoreMut or simlar.
        generator: &mut ElementIdGenerator,
        resolver_set: impl ResolverSet,
    ) -> Self::LayoutInfo;

    fn layout_element<'a>(
        &'a self,
        state: &'a Context<App>,
        store: &'a ElementStore,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut Layout<'a, App>,
    );
}

impl<App> ElementSet<App> for ()
where
    App: Application,
{
    type LayoutInfo = ();

    fn get_element_count(&self) -> usize {
        0
    }

    fn create_layout_info(&mut self, _: &Context<App>, _: &mut ElementStore, _: &mut ElementIdGenerator, _: impl ResolverSet) {}

    fn layout_element<'a>(&'a self, _: &'a Context<App>, _: &'a ElementStore, _: &'a Self::LayoutInfo, _: &mut Layout<'a, App>) {}
}

impl<App, T, const N: usize> ElementSet<App> for [T; N]
where
    App: Application,
    T: Element<App>,
{
    type LayoutInfo = [T::LayoutInfo; N];

    fn get_element_count(&self) -> usize {
        N
    }

    fn create_layout_info(
        &mut self,
        state: &Context<App>,
        store: &mut ElementStore,
        generator: &mut ElementIdGenerator,
        mut resolver_set: impl ResolverSet,
    ) -> Self::LayoutInfo {
        std::array::from_fn(|index| {
            resolver_set.with_index(index, |resolver| {
                self[index].create_layout_info(
                    state,
                    store.get_or_create_child_store(index as u64, generator),
                    generator,
                    resolver,
                )
            })
        })
    }

    fn layout_element<'a>(
        &'a self,
        state: &'a Context<App>,
        store: &'a ElementStore,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut Layout<'a, App>,
    ) {
        for index in 0..N {
            self[index].layout_element(state, store.child_store(index as u64), &layout_info[index], layout);
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
                        self.{index}.create_layout_info(state, store.get_or_create_child_store({index}, generator), generator, resolver)
                    }}), "
                )
            })
            .collect();

        let layout_elements: String = (0..number_of_elements)
            .map(|index| format!("self.{index}.layout_element(state, store.child_store({index}), &layout_info.{index}, layout);"))
            .collect();

        crabtime::output! {
            impl<App, {{element_list}}> ElementSet<App> for ({{element_list}})
            where
                App: Application,
                {{bounds}}
            {
                type LayoutInfo = ({{layout_info}});

                fn get_element_count(&self) -> usize {
                    {{number_of_elements}}
                }

                fn create_layout_info(
                    &mut self,
                    state: &Context<App>,
                    store: &mut ElementStore,
                    generator: &mut ElementIdGenerator,
                    mut resolver_set: impl ResolverSet,
                ) -> Self::LayoutInfo {
                    ({{create_layout_infos}})
                }

                fn layout_element<'a>(
                    &'a self,
                    state: &'a Context<App>,
                    store: &'a ElementStore,
                    layout_info: &'a Self::LayoutInfo,
                    layout: &mut Layout<'a, App>,
                ) {
                    {{layout_elements}}
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
    pub fn new(element: E) -> Self {
        Self {
            element,
            layout_info: None,
            _marker: PhantomData,
        }
    }
}

impl<App, E> Element<App> for ErasedElement<App, E>
where
    App: Application,
    E: Element<App>,
    E::LayoutInfo: 'static,
{
    type LayoutInfo = ();

    fn create_layout_info(
        &mut self,
        state: &Context<App>,
        store: &mut ElementStore,
        generator: &mut ElementIdGenerator,
        resolver: &mut Resolver,
    ) {
        let layout_info = self.element.create_layout_info(state, store, generator, resolver);
        self.layout_info = Some(layout_info);
    }

    fn layout_element<'a>(
        &'a self,
        state: &'a Context<App>,
        store: &'a ElementStore,
        _: &'a Self::LayoutInfo,
        layout: &mut Layout<'a, App>,
    ) {
        let layout_info = self.layout_info.as_ref().expect("no layout created");
        self.element.layout_element(state, store, layout_info, layout);
    }
}

pub type ElementBox<App: Application> = Box<dyn Element<App, LayoutInfo = ()>>;
