use rust_state::{Selector, State};

use crate::application::Application;
use crate::element::Element;
use crate::element::store::{ElementStore, ElementStoreMut};
use crate::layout::area::PartialArea;
use crate::layout::{Resolver, Resolvers, WindowLayout, with_single_resolver};

struct CellResolvers<'a, App: Application> {
    initial_available_area: PartialArea,
    cell_size: f32,
    gaps: f32,
    used_height: f32,
    text_layouter: &'a App::TextLayouter,
}

impl<'a, App> CellResolvers<'a, App>
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

impl<'a, App> Resolvers<'a, App> for CellResolvers<'a, App>
where
    App: Application,
{
    fn for_index(&self, index: usize) -> Resolver<'a, App> {
        let cell_area = PartialArea {
            left: self.initial_available_area.left + self.gaps * index as f32 + self.cell_size * index as f32,
            top: self.initial_available_area.top,
            width: self.cell_size,
            height: self.initial_available_area.height,
        };

        Resolver::new(cell_area, self.gaps, self.text_layouter)
    }

    fn give_back(&mut self, resolver: Resolver<'a, App>) {
        self.used_height = self.used_height.max(resolver.get_used_height());
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
    Children: Element<App>,
{
    type LayoutInfo = Children::LayoutInfo;

    fn create_layout_info(&mut self, state: &State<App>, store: ElementStoreMut, resolvers: &mut dyn Resolvers<App>) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            let gaps = *state.get(&self.gaps);
            let element_count = self.children.get_element_count(state);

            let available_area = resolver.push_available_area();
            let available_width = available_area.width - gaps * element_count.saturating_sub(1) as f32;
            // TODO: This is obviously a divide by 0 if we don't have any child elements.
            // Should be protected somehow.
            let cell_size = available_width / element_count as f32;
            let mut resolvers = CellResolvers::new(available_area, cell_size, gaps, resolver.get_text_layouter());

            let layout_info = self.children.create_layout_info(state, store, &mut resolvers as _);

            let used_height = resolvers.used_height;
            resolver.commit_used_height(used_height);

            layout_info
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<App>,
        store: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, App>,
    ) {
        self.children.lay_out(state, store, layout_info, layout);
    }
}
