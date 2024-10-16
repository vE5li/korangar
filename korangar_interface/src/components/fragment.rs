use rust_state::{Context, Selector};

use crate::application::Application;
use crate::element::store::{ElementStore, ElementStoreMut};
use crate::element::{Element, ElementSet};
use crate::layout::{Resolver, WindowLayout};

pub struct Fragment<A, B, Children> {
    gaps: A,
    border: B,
    children: Children,
}

impl<A, B, Children> Fragment<A, B, Children> {
    /// This function is supposed to be called from a component macro and not
    /// intended to be called manually.
    #[inline(always)]
    pub fn component_new(gaps: A, border: B, children: Children) -> Self {
        Self { gaps, border, children }
    }
}

impl<App, A, B, Children> Element<App> for Fragment<A, B, Children>
where
    App: Application,
    A: Selector<App, f32>,
    B: Selector<App, f32>,
    Children: ElementSet<App>,
{
    type LayoutInfo = Children::LayoutInfo;

    fn create_layout_info(
        &mut self,
        state: &Context<App>,
        store: ElementStoreMut<'_>,
        resolver: &mut Resolver<'_, App>,
    ) -> Self::LayoutInfo {
        let (_, children) = resolver.with_derived(*state.get(&self.gaps), *state.get(&self.border), |resolver| {
            self.children.create_layout_info(state, store, resolver)
        });

        children
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a Context<App>,
        store: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, App>,
    ) {
        self.children.lay_out(state, store, layout_info, layout);
    }
}
