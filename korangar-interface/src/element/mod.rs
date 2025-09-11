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
use crate::layout::{Resolver, ResolverSet, WindowLayout};

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
        layout: &mut WindowLayout<'a, App>,
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
        layout: &mut WindowLayout<'a, App>,
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

    fn lay_out<'a>(&'a self, _: &'a Context<App>, _: ElementStore<'a>, _: &'a Self::LayoutInfo, _: &mut WindowLayout<'a, App>) {}
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
        layout: &mut WindowLayout<'a, App>,
    ) {
        for index in 0..N {
            self[index].lay_out(state, store.child_store(index as u64), &layout_info[index], layout);
        }
    }
}

macro_rules! impl_element_set_for {
    ($head:ident) => {
        impl_element_set_for!(# $head );
    };
    ($head:ident, $($tail:ident),*) => {
        impl_element_set_for!(# $head, $($tail),* );
        impl_element_set_for!( $($tail),* );
    };
    (# $($ty:ident),*) => {
        impl<App, $($ty),*> ElementSet<App> for ($($ty,)*)
        where
            App: Application,
            $($ty: Element<App>,)*
        {
            type LayoutInfo = ($($ty::LayoutInfo,)*);

            fn get_element_count(&self, _: &Context<App>) -> usize {
                ${count($ty)}
            }

            fn create_layout_info(
                &mut self,
                state: &Context<App>,
                mut store: ElementStoreMut<'_>,
                mut resolver_set: impl ResolverSet<'_, App>,
            ) -> Self::LayoutInfo {
                ($(
                    resolver_set.with_index(${index()}, |resolver| {
                        // Redundant binding only here to iterate $ty so ${index()} knows the context.
                        let field: &mut $ty = &mut self.${index()};
                        field.create_layout_info(state, store.child_store(${index()}), resolver)
                    }),
                )*)
            }

            fn lay_out<'a>(
                &'a self,
                state: &'a Context<App>,
                store: ElementStore<'a>,
                layout_info: &'a Self::LayoutInfo,
                layout: &mut WindowLayout<'a, App>,
            ) {
                $(
                    // Redundant binding only here to iterate $ty so ${index()} knows the context.
                    let field: &$ty = &self.${index()};
                    field.lay_out(state, store.child_store(${index()}), &layout_info.${index()}, layout);
                )*
            }
        }
    };
}

// Implement `ElementSet` for tuples up to 64 elements.
impl_element_set_for!(
    T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28,
    T29, T30, T31, T32, T33, T34, T35, T36, T37, T38, T39, T40, T41, T42, T43, T44, T45, T46, T47, T48, T49, T50, T51, T52, T53, T54, T55,
    T56, T57, T58, T59, T60, T61, T62, T63, T64, T65, T66, T67, T68, T69, T70, T71, T72, T73, T74, T75, T76, T77, T78, T79, T80, T81, T82
);

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

    fn lay_out<'a>(
        &'a self,
        state: &'a Context<App>,
        store: ElementStore<'a>,
        _: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, App>,
    ) {
        let layout_info = self.layout_info.as_ref().expect("no layout created");
        self.element.lay_out(state, store, layout_info, layout);
    }
}

pub type ElementBox<App> = Box<dyn Element<App, LayoutInfo = ()>>;
