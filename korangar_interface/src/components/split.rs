use rust_state::Context;

use crate::application::Appli;
use crate::element::id::ElementIdGenerator;
use crate::element::store::ElementStore;
use crate::element::{Element, ElementSet, ResolverSet};
use crate::layout::area::{Area, PartialArea};
use crate::layout::{Layout, Resolver};

pub struct Split<Children> {
    pub children: Children,
}

struct CellResolverSet<'a> {
    resolver: &'a mut Resolver,
    initial_available_area: PartialArea,
    cell_size: f32,
}

impl<'a> CellResolverSet<'a> {
    pub fn new(resolver: &'a mut Resolver, cell_size: f32) -> Self {
        let initial_available_area = resolver.push_available_area();

        Self {
            resolver,
            initial_available_area,
            cell_size,
        }
    }
}

impl ResolverSet for CellResolverSet<'_> {
    fn with_index<C>(&mut self, index: usize, f: impl FnMut(&mut Resolver) -> C) -> C {
        let cell_area = PartialArea {
            x: self.initial_available_area.x + self.cell_size * index as f32,
            y: self.initial_available_area.y,
            width: self.cell_size,
            height: self.initial_available_area.height,
        };

        self.resolver.with_derived_custom(cell_area, f)
    }
}

impl<App, Children> Element<App> for Split<Children>
where
    App: Appli,
    Children: ElementSet<App>,
{
    type Layouted = Children::Layouted;

    fn make_layout(
        &mut self,
        state: &Context<App>,
        store: &mut ElementStore,
        generator: &mut ElementIdGenerator,
        resolver: &mut Resolver,
    ) -> Self::Layouted {
        let cell_size = resolver.push_available_area().width / self.children.get_element_count() as f32;
        let resolver_set = CellResolverSet::new(resolver, cell_size);

        self.children.make_layout(state, store, generator, resolver_set)
    }

    fn create_layout<'a>(
        &'a self,
        state: &'a Context<App>,
        store: &'a ElementStore,
        layouted: &'a Self::Layouted,
        layout: &mut Layout<'a, App>,
    ) {
        self.children.create_layout(state, store, layouted, layout);
    }
}
