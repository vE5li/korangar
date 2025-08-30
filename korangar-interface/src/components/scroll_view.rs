use std::cell::RefCell;

use num::Signed;
use rust_state::Context;

use crate::application::Application;
use crate::element::store::{ElementStore, ElementStoreMut, Persistent, PersistentExt};
use crate::element::{Element, ElementSet};
use crate::layout::area::Area;
use crate::layout::{Resolver, ScrollHandler, WindowLayout};
use crate::prelude::EventQueue;

#[derive(Default)]
struct PersistentDataInner {
    scroll: f32,
    max_scroll: f32,
}

#[derive(Default)]
pub struct PersistentData {
    inner: RefCell<PersistentDataInner>,
}

impl<App> ScrollHandler<App> for PersistentData
where
    App: Application,
{
    fn handle_scroll(&self, _: &Context<App>, _: &mut EventQueue<App>, _: <App as Application>::Position, delta: f32) -> bool {
        let mut inner = self.inner.borrow_mut();

        // Don't try to scroll if its already at the minimum or maximum scroll value.
        if delta.is_negative() && inner.scroll >= inner.max_scroll || delta.is_positive() && inner.scroll <= 0.0 {
            return false;
        }

        inner.scroll = (inner.scroll - delta).max(0.0).min(inner.max_scroll);

        true
    }
}

pub struct ScrollViewLayoutInfo<L> {
    area: Area,
    children: L,
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
            let scroll = persistent.inner.borrow().scroll;

            // In case that we need to resolve twice we don't want to start with the same
            // resolver state as the first iteration, so we clone it here and adding it back
            // as soon as a correct layout was found. This is a little bit
            // ugly and might be improved in the future.
            let mut cloned_resolver = resolver.clone();

            // HACK: Since this loop might run multiple times we need a new store every
            // time. This is a bit wasteful and I would like to solve this more
            // elegantly.
            let child_store = store.child_store(0);

            let (area, children_height, layout_info) = cloned_resolver.with_derived_scrolled(scroll, |resolver| {
                self.children.create_layout_info(state, child_store, resolver)
            });

            let persistent = self.get_persistent_data(&store, ());
            let mut inner = persistent.inner.borrow_mut();

            let max_scroll = (children_height - area.height).max(0.0);

            // Check if the scroll is in bounds. If it is, we can just return, otherwise we
            // need to adjust it and create the layout again.
            if inner.scroll > max_scroll {
                inner.scroll = max_scroll;
                continue;
            } else if inner.scroll < 0.0 {
                inner.scroll = 0.0;

                continue;
            }

            inner.max_scroll = max_scroll;

            *resolver = cloned_resolver;

            return ScrollViewLayoutInfo {
                area,
                children: layout_info,
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
            layout.add_scroll_area(layout_info.area, persistent);
        }

        layout.with_clip_layer(layout_info.area, |layout| {
            layout.with_layer(|layout| {
                // HACK: We need to do the same as in `create_layout_info`.
                self.children.lay_out(state, store.child_store(0), &layout_info.children, layout);
            });
        });
    }
}
