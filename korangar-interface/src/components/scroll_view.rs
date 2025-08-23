use std::cell::RefCell;

use rust_state::Context;

use crate::application::Application;
use crate::element::store::{ElementStore, ElementStoreMut, Persistent, PersistentExt};
use crate::element::{Element, ElementSet};
use crate::layout::area::Area;
use crate::layout::{Resolver, WindowLayout};

#[derive(Default)]
pub struct PersistentData {
    scroll: RefCell<f32>,
    // animation_state: AnimationState,
}

pub struct ScrollViewLayoutInfo<L> {
    area: Area,
    children: L,
    max_scroll: f32,
}

pub struct ScrollView<Children> {
    children: Children,
}

impl<Children> ScrollView<Children> {
    /// This function is supposed to be called from a component macro and not
    /// intended to be called manually.
    #[inline(always)]
    pub fn component_new(children: Children) -> Self {
        Self { children }
    }
}

impl<Children> Persistent for ScrollView<Children> {
    type Data = PersistentData;
}

impl<App, Children> Element<App> for ScrollView<Children>
where
    App: Application,
    Children: ElementSet<App>,
{
    type LayoutInfo = ScrollViewLayoutInfo<Children::LayoutInfo>;

    fn create_layout_info(
        &mut self,
        state: &Context<App>,
        mut store: ElementStoreMut<'_>,
        resolver: &mut Resolver<'_, App>,
    ) -> Self::LayoutInfo {
        loop {
            let persistent = self.get_persistent_data(&store, ());
            let current_scroll = *persistent.scroll.borrow();

            // In case that we need to resolve twice we don't want to start with the same
            // resolver state as the first iteration, so we clone it here and assing it back
            // as soon as a correct layout was found. This is a little bit
            // ugly and might be improved in the future.
            let mut cloned_resolver = resolver.clone();

            // HACK: Since this loop might run multiple times we need a new store every
            // time. This is a bit wasteful and I would like to solve this more
            // elegantly.
            let child_store = store.child_store(0);

            let (area, children_height, layout_info) = cloned_resolver.with_derived_scrolled(current_scroll, |resolver| {
                self.children.create_layout_info(state, child_store, resolver)
            });

            let persistent = self.get_persistent_data(&store, ());
            let mut current_scroll = persistent.scroll.borrow_mut();

            let max_scroll = (children_height - area.height).max(0.0);

            // Check if the scroll is in bounds. If it is, we can just return, otherwise we
            // need to adjust it and create the layout again.
            if *current_scroll > max_scroll {
                *current_scroll = max_scroll;
                continue;
            } else if *current_scroll < 0.0 {
                *current_scroll = 0.0;

                continue;
            }

            *resolver = cloned_resolver;

            return ScrollViewLayoutInfo {
                area,
                children: layout_info,
                max_scroll,
            };
        }
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a Context<App>,
        store: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, App>,
    ) {
        let persistent = self.get_persistent_data(&store, ());

        if layout_info.area.check().dont_mark().run(layout) {
            layout.add_scroll_area(layout_info.area, layout_info.max_scroll, &persistent.scroll);
        }

        layout.with_clip_layer(layout_info.area, |layout| {
            layout.with_layer(|layout| {
                // HACK: We need to do the same as in `create_layout_info`.
                self.children.lay_out(state, store.child_store(0), &layout_info.children, layout);
            });
        });
    }
}
