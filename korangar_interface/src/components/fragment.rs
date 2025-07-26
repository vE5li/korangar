use rust_state::{Context, Selector};

use crate::application::Application;
use crate::element::id::ElementIdGenerator;
use crate::element::store::ElementStore;
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
        store: &mut ElementStore,
        generator: &mut ElementIdGenerator,
        resolver: &mut Resolver,
    ) -> Self::LayoutInfo {
        let (_, children) = resolver.with_derived(*state.get(&self.gaps), *state.get(&self.border), |resolver| {
            self.children.create_layout_info(state, store, generator, resolver)
        });

        children
    }

    fn layout_element<'a>(
        &'a self,
        state: &'a Context<App>,
        store: &'a ElementStore,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut Layout<'a, App>,
    ) {
        self.children.layout_element(state, store, layout_info, layout);
    }
}
