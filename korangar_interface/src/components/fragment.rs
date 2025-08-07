use rust_state::{Context, Selector};

use crate::application::Application;
use crate::element::store::{ElementStore, ElementStoreMut};
use crate::element::{Element, ElementSet};
use crate::layout::{Layout, Resolver};

pub struct Fragment<A, B, Children> {
    pub gaps: A,
    pub border: B,
    pub children: Children,
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
        layout: &mut Layout<'a, App>,
    ) {
        self.children.lay_out(state, store, layout_info, layout);
    }
}
