use rust_state::{Context, Selector};

use crate::application::Application;
use crate::element::id::ElementIdGenerator;
use crate::element::store::ElementStore;
use crate::element::{Element, ElementSet, ResolverSet};
use crate::layout::area::PartialArea;
use crate::layout::{Layout, Resolver};

struct CellResolverSet {
    initial_available_area: PartialArea,
    cell_size: f32,
    gaps: f32,
    used_height: f32,
}

impl CellResolverSet {
    pub fn new(initial_available_area: PartialArea, cell_size: f32, gaps: f32) -> Self {
        Self {
            initial_available_area,
            cell_size,
            gaps,
            used_height: 0.0,
        }
    }
}

impl ResolverSet for &mut CellResolverSet {
    fn with_index<C>(&mut self, index: usize, mut f: impl FnMut(&mut Resolver) -> C) -> C {
        let cell_area = PartialArea {
            left: self.initial_available_area.left + self.gaps * index as f32 + self.cell_size * index as f32,
            top: self.initial_available_area.top,
            width: self.cell_size,
            height: self.initial_available_area.height,
        };

        let mut resolver = Resolver::new(cell_area, self.gaps);

        let layout_info = f(&mut resolver);

        self.used_height = self.used_height.max(resolver.get_used_height());

        layout_info
    }
}

pub struct Split<A, Children> {
    pub gaps: A,
    pub children: Children,
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
        store: &mut ElementStore,
        generator: &mut ElementIdGenerator,
        resolver: &mut Resolver,
    ) -> Self::LayoutInfo {
        let gaps = *state.get(&self.gaps);
        let element_count = self.children.get_element_count();

        let available_area = resolver.push_available_area();
        let available_width = available_area.width - gaps * element_count.saturating_sub(1) as f32;
        // TODO: This is obviously a divide by 0 if we don't have any child elements.
        // Should be protected somehow.
        let cell_size = available_width / element_count as f32;
        let mut resolver_set = CellResolverSet::new(available_area, cell_size, gaps);

        let layout_info = self.children.create_layout_info(state, store, generator, &mut resolver_set);

        let used_height = resolver_set.used_height;
        resolver.commit_used_height(used_height);

        layout_info
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
