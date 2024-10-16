use rust_state::{Context, Selector};

use crate::application::Application;
use crate::element::store::{ElementStore, ElementStoreMut};
use crate::element::{Element, ElementSet};
use crate::layout::area::PartialArea;
use crate::layout::{Resolver, ResolverSet, WindowLayout};

struct CellResolverSet<'a, App: Application> {
    initial_available_area: PartialArea,
    cell_size: f32,
    gaps: f32,
    used_height: f32,
    text_layouter: &'a App::TextLayouter,
}

impl<'a, App> CellResolverSet<'a, App>
where
    App: Application,
{
    pub fn new(initial_available_area: PartialArea, cell_size: f32, gaps: f32, text_layouter: &'a App::TextLayouter) -> Self {
        Self {
            initial_available_area,
            cell_size,
            gaps,
            used_height: 0.0,
            text_layouter,
        }
    }
}

impl<'a, App> ResolverSet<'a, App> for &mut CellResolverSet<'a, App>
where
    App: Application,
{
    fn with_index<C>(&mut self, index: usize, mut f: impl FnMut(&mut Resolver<'a, App>) -> C) -> C {
        let cell_area = PartialArea {
            left: self.initial_available_area.left + self.gaps * index as f32 + self.cell_size * index as f32,
            top: self.initial_available_area.top,
            width: self.cell_size,
            height: self.initial_available_area.height,
        };

        let mut resolver = Resolver::new(cell_area, self.gaps, self.text_layouter);

        let layout_info = f(&mut resolver);

        self.used_height = self.used_height.max(resolver.get_used_height());

        layout_info
    }
}

pub struct Split<A, Children> {
    gaps: A,
    children: Children,
}

impl<A, Children> Split<A, Children> {
    /// This function is supposed to be called from a component macro and not
    /// intended to be called manually.
    #[inline(always)]
    pub fn component_new(gaps: A, children: Children) -> Self {
        Self { gaps, children }
    }
}

impl<App, A, Children> Element<App> for Split<A, Children>
where
    App: Application,
    A: Selector<App, f32>,
    Children: ElementSet<App>,
{
    type LayoutInfo = Children::LayoutInfo;

    fn create_layout_info(
        &mut self,
        state: &Context<App>,
        store: ElementStoreMut<'_>,
        resolver: &mut Resolver<'_, App>,
    ) -> Self::LayoutInfo {
        let gaps = *state.get(&self.gaps);
        let element_count = self.children.get_element_count(state);

        let available_area = resolver.push_available_area();
        let available_width = available_area.width - gaps * element_count.saturating_sub(1) as f32;
        // TODO: This is obviously a divide by 0 if we don't have any child elements.
        // Should be protected somehow.
        let cell_size = available_width / element_count as f32;
        let mut resolver_set = CellResolverSet::new(available_area, cell_size, gaps, resolver.get_text_layouter());

        let layout_info = self.children.create_layout_info(state, store, &mut resolver_set);

        let used_height = resolver_set.used_height;
        resolver.commit_used_height(used_height);

        layout_info
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
