mod prototype;

use id::ElementIdGenerator;
pub use interface_macros::PrototypeElement;
use rust_state::Context;
use store::ElementStore;

pub use self::prototype::*;
use crate::application::Appli;
use crate::layout::area::Area;
use crate::layout::{Layout, Resolver};

pub mod id {
    #[derive(Clone, Copy, PartialEq, Eq)]
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

// TODO: Change name
pub struct DefaultLayouted {
    pub area: Area,
}

pub struct DefaultLayoutedSet<T> {
    pub area: Area,
    pub children: T,
}

pub trait Element<App: Appli> {
    // TODO: Change name
    type Layouted = DefaultLayouted;

    // TODO: Change name
    fn make_layout(
        &mut self,
        state: &Context<App>,
        store: &mut ElementStore,
        generator: &mut ElementIdGenerator,
        resolver: &mut Resolver,
    ) -> Self::Layouted;

    fn create_layout<'a>(
        &'a self,
        state: &'a Context<App>,
        store: &'a ElementStore,
        layouted: &'a Self::Layouted,
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

pub trait ElementSet<App: Appli> {
    // TODO: Change name
    type Layouted;

    fn get_element_count(&self) -> usize;

    fn make_layout(
        &mut self,
        state: &Context<App>,
        store: &mut ElementStore,
        generator: &mut ElementIdGenerator,
        resolver_set: impl ResolverSet,
    ) -> Self::Layouted;

    fn create_layout<'a>(
        &'a self,
        state: &'a Context<App>,
        store: &'a ElementStore,
        layouted: &'a Self::Layouted,
        layout: &mut Layout<'a, App>,
    );
}

impl<App> ElementSet<App> for ()
where
    App: Appli,
{
    type Layouted = ();

    fn get_element_count(&self) -> usize {
        0
    }

    fn make_layout(&mut self, _: &Context<App>, _: &mut ElementStore, _: &mut ElementIdGenerator, _: impl ResolverSet) {}

    fn create_layout<'a>(&'a self, _: &'a Context<App>, _: &'a ElementStore, _: &'a Self::Layouted, _: &mut Layout<'a, App>) {}
}

impl<App, T, const N: usize> ElementSet<App> for [T; N]
where
    App: Appli,
    T: Element<App>,
{
    type Layouted = [T::Layouted; N];

    fn get_element_count(&self) -> usize {
        N
    }

    fn make_layout(
        &mut self,
        state: &Context<App>,
        store: &mut ElementStore,
        generator: &mut ElementIdGenerator,
        mut resolver_set: impl ResolverSet,
    ) -> Self::Layouted {
        std::array::from_fn(|index| {
            resolver_set.with_index(index, |resolver| {
                self[index].make_layout(
                    state,
                    store.get_or_create_child_store(index as u64, generator),
                    generator,
                    resolver,
                )
            })
        })
    }

    fn create_layout<'a>(
        &'a self,
        state: &'a Context<App>,
        store: &'a ElementStore,
        layouted: &'a Self::Layouted,
        layout: &mut Layout<'a, App>,
    ) {
        for index in 0..N {
            self[index].create_layout(state, store.child_store(index as u64), &layouted[index], layout);
        }
    }
}

#[crabtime::function]
fn impl_element_set(up_to: usize) {
    for number_of_elements in 1..=up_to {
        let elements = (0..number_of_elements).map(|index| format!("T{index}"));
        let element_list: String = elements.clone().map(|element| format!("{element}, ")).collect();
        let layouted: String = elements.clone().map(|element| format!("{element}::Layouted, ")).collect();
        let bounds: String = elements.map(|element| format!("{element}: Element<App>, ")).collect();

        let make_layouts: String = (0..number_of_elements)
            .map(|index| {
                format!(
                    "
                    resolver_set.with_index({index}, |resolver| {{
                        self.{index}.make_layout(state, store.get_or_create_child_store({index}, generator), generator, resolver)
                    }}), "
                )
            })
            .collect();

        let create_layouts: String = (0..number_of_elements)
            .map(|index| format!("self.{index}.create_layout(state, store.child_store({index}), &layouted.{index}, layout);"))
            .collect();

        crabtime::output! {
            impl<App, {{element_list}}> ElementSet<App> for ({{element_list}})
            where
                App: Appli,
                {{bounds}}
            {
                type Layouted = ({{layouted}});

                fn get_element_count(&self) -> usize {
                    {{number_of_elements}}
                }

                fn make_layout(
                    &mut self,
                    state: &Context<App>,
                    store: &mut ElementStore,
                    generator: &mut ElementIdGenerator,
                    mut resolver_set: impl ResolverSet,
                ) -> Self::Layouted {
                    ({{make_layouts}})
                }

                fn create_layout<'a>(
                    &'a self,
                    state: &'a Context<App>,
                    store: &'a ElementStore,
                    layouted: &'a Self::Layouted,
                    layout: &mut Layout<'a, App>,
                ) {
                    {{create_layouts}}
                }
            }
        }
    }
}

// Implement `ElementSet` for tuples up to 64 elements.
impl_element_set!(64);
