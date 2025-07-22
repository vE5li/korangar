use std::cell::RefCell;

use rust_state::Context;

use crate::application::Application;
use crate::element::id::ElementIdGenerator;
use crate::element::store::{ElementStore, Persistent, PersistentExt};
use crate::element::{Element, ElementSet};
use crate::layout::area::Area;
use crate::layout::{HeightBound, Layout, Resolver};

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
    pub children: Children,
    pub height_bound: HeightBound,
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
        store: &mut ElementStore,
        generator: &mut ElementIdGenerator,
        resolver: &mut Resolver,
    ) -> Self::LayoutInfo {
        loop {
            let persistent = self.get_persistent_data(store, ());
            let current_scroll = *persistent.scroll.borrow();

            // In case that we need to resolve twice we don't want to start with the same
            // resolver state as the first iteration, so we clone it here and assing it back
            // as soon as a correct layout was found. This is a little bit
            // ugly and might be improved in the future.
            let mut cloned_resolver = resolver.clone();

            let (area, children_height, layout_info) =
                cloned_resolver.with_derived_scrolled(current_scroll, self.height_bound, |resolver| {
                    self.children.create_layout_info(state, store, generator, resolver)
                });

            let persistent = self.get_persistent_data(store, ());
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

    fn layout_element<'a>(
        &'a self,
        state: &'a Context<App>,
        store: &'a ElementStore,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut Layout<'a, App>,
    ) {
        let persistent = self.get_persistent_data(store, ());

        if self.height_bound == HeightBound::Unbound {
            println!("unbound scroll views don't do anything");
        }

        layout.with_clip_layer(layout_info.area, |layout| {
            layout.with_layer(|layout| {
                self.children.layout_element(state, store, &layout_info.children, layout);
            });
        });

        if layout.is_area_hovered(layout_info.area) {
            layout.add_scroll_area(layout_info.area, layout_info.max_scroll, &persistent.scroll);
        }
    }
}
