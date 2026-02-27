use rust_state::{Selector, State};

use crate::application::Application;
use crate::element::Element;
use crate::element::store::{ElementStore, ElementStoreMut};
use crate::layout::{Resolvers, WindowLayout};

pub enum EitherLayoutInfo<OnTrue, OnFalse> {
    OnTrue(OnTrue),
    OnFalse(OnFalse),
}

pub struct Either<A, OnTrue, OnFalse> {
    selector: A,
    on_true: OnTrue,
    on_false: OnFalse,
}

impl<A, OnTrue, OnFalse> Either<A, OnTrue, OnFalse> {
    /// This function is supposed to be called from a component macro and not
    /// intended to be called manually.
    #[inline(always)]
    pub fn component_new(selector: A, on_true: OnTrue, on_false: OnFalse) -> Self {
        Self {
            selector,
            on_true,
            on_false,
        }
    }
}

impl<App, A, OnTrue, OnFalse> Element<App> for Either<A, OnTrue, OnFalse>
where
    App: Application,
    A: Selector<App, bool>,
    OnTrue: Element<App>,
    OnFalse: Element<App>,
{
    type LayoutInfo = EitherLayoutInfo<OnTrue::LayoutInfo, OnFalse::LayoutInfo>;

    fn get_element_count(&self, state: &State<App>) -> usize {
        match *state.get(&self.selector) {
            true => self.on_true.get_element_count(state),
            false => self.on_false.get_element_count(state),
        }
    }

    fn create_layout_info(&mut self, state: &State<App>, store: ElementStoreMut, resolvers: &mut dyn Resolvers<App>) -> Self::LayoutInfo {
        match *state.get(&self.selector) {
            true => EitherLayoutInfo::OnTrue(self.on_true.create_layout_info(state, store, resolvers)),
            false => EitherLayoutInfo::OnFalse(self.on_false.create_layout_info(state, store, resolvers)),
        }
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<App>,
        store: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, App>,
    ) {
        match layout_info {
            EitherLayoutInfo::OnTrue(layout_info) => self.on_true.lay_out(state, store, layout_info, layout),
            EitherLayoutInfo::OnFalse(layout_info) => self.on_false.lay_out(state, store, layout_info, layout),
        }
    }
}
