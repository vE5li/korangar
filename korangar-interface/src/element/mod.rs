pub mod id;
mod state;
pub mod store;

use std::marker::PhantomData;

pub use interface_macros::StateElement;
use rust_state::State;
use store::{ElementStore, ElementStoreMut};

pub use self::state::*;
use crate::application::Application;
use crate::layout::area::Area;
use crate::layout::{Resolvers, WindowLayout, with_nth_resolver};

pub struct BaseLayoutInfo {
    pub area: Area,
}

pub struct DefaultLayoutInfo<App: Application> {
    pub area: Area,
    pub font_size: App::FontSize,
}

/// An element in the user interface.
///
/// Elements may contain other elements themselves.
pub trait Element<App: Application> {
    /// Data passed from [`create_layout_info`](Element::create_layout_info) to
    /// [`lay_out`](Element::lay_out).
    ///
    /// This is mostly used to store the allocated area of the element and its
    /// children.
    type LayoutInfo = DefaultLayoutInfo<App>;

    /// Get the number of elements that will be layed out by this element.
    ///
    /// This information is important for components that need to know the
    /// number of children to render correctly, e.g. `split` and `collapsible`.
    ///
    /// In the vast majority of cases, this will be `1`. Standard components
    /// such as `button`, `text`, `text_box`, etc. only render themselves.
    ///
    /// Other componets, like `split`, `scoll_view`, and `fragment`, have
    /// children that they render *but* the children are internal to the
    /// element, thus it still looks like one element from the outside.
    ///
    /// Types that *don't* look like one element from the outside include:
    /// - Tuples of elemets: A tuple of elements will always have an element
    ///   count equal to its length
    /// - Arrays of elements: Similarly, the number of elements is equal to the
    ///   length of the array
    /// - `either` component: Will return the number of elements of `on_true` or
    ///   `on_false` based on the selector. That way, components like `split`
    ///   will always render correctly with `either` as a child.
    fn get_element_count(&self, _: &State<App>) -> usize {
        1
    }

    /// Create the data used by `lay_out` to render the element.
    ///
    /// This typically involves (loosely in order):
    /// - Updating or creating persistent data in the element store
    /// - Updating element state based on the application state
    /// - Allocating space for the element using the resolver
    /// - Laying out text to get the dimensions and font size
    /// - Doing the same for children elements
    fn create_layout_info(&mut self, state: &State<App>, store: ElementStoreMut, resolvers: &mut dyn Resolvers<App>) -> Self::LayoutInfo;

    /// Add the element to the [layout](WindowLayout).
    ///
    /// This typically involves:
    /// - Checking if the element is hovered
    /// - Adding render instructions to the layout
    /// - Adding input handlers to the layout
    fn lay_out<'a>(
        &'a self,
        state: &'a State<App>,
        store: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, App>,
    );
}

/// Empty element to use as a placeholder.
///
/// In contrast to [`()`], which has an element count of `0`, it has an element
/// count of `1`.
pub struct EmptyElement;

impl<App> Element<App> for EmptyElement
where
    App: Application,
{
    type LayoutInfo = ();

    fn create_layout_info(&mut self, _: &State<App>, _: ElementStoreMut, _: &mut dyn Resolvers<App>) -> Self::LayoutInfo {}

    fn lay_out<'a>(&'a self, _: &'a State<App>, _: ElementStore<'a>, _: &'a Self::LayoutInfo, _: &mut WindowLayout<'a, App>) {}
}

/// Element that stores its own layout info, rather than letting the parent
/// element store it. Thus, the type of [`LayoutInfo`](Element::LayoutInfo) is
/// [`()`].
///
/// The layout info always having the same type enables [`ElementBox`] to store
/// any erased element using dynamic dispatch.
///
/// See [`ErasedElement::new`] for creating an [`ElementBox`].
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
    /// Creates a new [`ElementBox`] with an [`ErasedElement`] inside from any
    /// element.
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

    fn create_layout_info(&mut self, state: &State<App>, store: ElementStoreMut, resolvers: &mut dyn Resolvers<App>) {
        let layout_info = self.element.create_layout_info(state, store, resolvers);
        self.layout_info = Some(layout_info);
    }

    fn lay_out<'a>(&'a self, state: &'a State<App>, store: ElementStore<'a>, _: &'a Self::LayoutInfo, layout: &mut WindowLayout<'a, App>) {
        let layout_info = self.layout_info.as_ref().expect("no layout created");
        self.element.lay_out(state, store, layout_info, layout);
    }
}

/// Fully erased element stored on the heap. See [`ErasedElement`].
pub type ElementBox<App> = Box<dyn Element<App, LayoutInfo = ()>>;

impl<App> Element<App> for ()
where
    App: Application,
{
    type LayoutInfo = ();

    fn get_element_count(&self, _: &State<App>) -> usize {
        0
    }

    fn create_layout_info(&mut self, _: &State<App>, _: ElementStoreMut, _: &mut dyn Resolvers<App>) {}

    fn lay_out<'a>(&'a self, _: &'a State<App>, _: ElementStore<'a>, _: &'a Self::LayoutInfo, _: &mut WindowLayout<'a, App>) {}
}

impl<App, T, const N: usize> Element<App> for [T; N]
where
    App: Application,
    T: Element<App>,
{
    type LayoutInfo = [T::LayoutInfo; N];

    fn get_element_count(&self, _: &State<App>) -> usize {
        N
    }

    fn create_layout_info(
        &mut self,
        state: &State<App>,
        mut store: ElementStoreMut,
        resolvers: &mut dyn Resolvers<App>,
    ) -> Self::LayoutInfo {
        std::array::from_fn(|index| {
            with_nth_resolver(resolvers, index, |resolver| {
                self[index].create_layout_info(state, store.child_store(index as u64), resolver as _)
            })
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<App>,
        store: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, App>,
    ) {
        for index in 0..N {
            self[index].lay_out(state, store.child_store(index as u64), &layout_info[index], layout);
        }
    }
}

macro_rules! impl_element_for_tuple {
    ($head:ident) => {
        impl_element_for_tuple!(# $head );
    };
    ($head:ident, $($tail:ident),*) => {
        impl_element_for_tuple!(# $head, $($tail),* );
        impl_element_for_tuple!( $($tail),* );
    };
    (# $($ty:ident),*) => {
        impl<App, $($ty),*> Element<App> for ($($ty,)*)
        where
            App: Application,
            $($ty: Element<App>,)*
        {
            type LayoutInfo = ($($ty::LayoutInfo,)*);

            fn get_element_count(&self, _: &State<App>) -> usize {
                ${count($ty)}
            }

            fn create_layout_info(
                &mut self,
                state: &State<App>,
                mut store: ElementStoreMut,
                resolvers: &mut dyn Resolvers<App>,
            ) -> Self::LayoutInfo {
                ($(
                    with_nth_resolver(resolvers, ${index()}, |resolver| {
                        // Redundant binding only here to iterate $ty so ${index()} knows the context.
                        let field: &mut $ty = &mut self.${index()};
                        field.create_layout_info(state, store.child_store(${index()}), resolver)
                    }),
                )*)
            }

            fn lay_out<'a>(
                &'a self,
                state: &'a State<App>,
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

impl_element_for_tuple!(
    T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27, T28,
    T29, T30, T31, T32, T33, T34, T35, T36, T37, T38, T39, T40, T41, T42, T43, T44, T45, T46, T47, T48, T49, T50, T51, T52, T53, T54, T55,
    T56, T57, T58, T59, T60, T61, T62, T63, T64, T65, T66, T67, T68, T69, T70, T71, T72, T73, T74, T75, T76, T77, T78, T79, T80, T81, T82
);
