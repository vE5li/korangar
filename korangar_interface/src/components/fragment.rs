use rust_state::Context;

use crate::application::Application;
use crate::element::id::ElementIdGenerator;
use crate::element::store::ElementStore;
use crate::element::{Element, ElementSet};
use crate::layout::{Layout, Resolver};

pub struct Fragment<Children> {
    pub children: Children,
}

impl<App, Children> Element<App> for Fragment<Children>
where
    App: Application,
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
        self.children.create_layout_info(state, store, generator, resolver)
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
