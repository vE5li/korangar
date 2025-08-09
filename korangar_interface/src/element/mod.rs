pub mod id;
mod state;
pub mod store;

use std::marker::PhantomData;

pub use interface_macros::StateElement;
use rust_state::Context;
use store::{ElementStore, ElementStoreMut};

pub use self::state::*;
use crate::application::Application;
use crate::layout::area::Area;
use crate::layout::{Layout, Resolver, ResolverSet};

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
