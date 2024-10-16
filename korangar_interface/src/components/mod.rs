pub mod text {
    use std::marker::PhantomData;

    use rust_state::{Context, RustState, Selector};

    use crate::application::Appli;
    use crate::element::Element;
    use crate::element::id::ElementIdGenerator;
    use crate::element::store::ElementStore;
    use crate::layout::alignment::{HorizontalAlignment, VerticalAlignment};
    use crate::layout::{Layout, Resolver};

    #[derive(RustState)]
    pub struct TextTheme<App>
    where
        App: Appli + 'static,
    {
        pub color: App::Color,
        pub height: f32,
        pub font_size: App::FontSize,
        pub horizontal_alignment: HorizontalAlignment,
        pub vertical_alignment: VerticalAlignment,
    }

    pub struct Text<T, A, B, C, D, E, F> {
        pub text_marker: PhantomData<T>,
        pub text: A,
        pub color: B,
        pub height: C,
        pub font_size: D,
        pub horizontal_alignment: E,
        pub vertical_alignment: F,
    }

    impl<App, T, A, B, C, D, E, F> Element<App> for Text<T, A, B, C, D, E, F>
    where
        App: Appli,
        T: AsRef<str> + 'static,
        A: Selector<App, T>,
        B: Selector<App, App::Color>,
        C: Selector<App, f32>,
        D: Selector<App, App::FontSize>,
        E: Selector<App, HorizontalAlignment>,
        F: Selector<App, VerticalAlignment>,
    {
        fn get_height(&self, state: &Context<App>, _: &ElementStore, _: &mut ElementIdGenerator, resolver: &mut Resolver) {
            let height = state.get(&self.height);
            resolver.with_height(*height);
        }

        fn create_layout<'a>(
            &'a self,
            state: &'a Context<App>,
            _: &'a ElementStore,
            _: &mut ElementIdGenerator,
            resolver: &mut Resolver,
            layout: &mut Layout<'a, App>,
        ) {
            let height = state.get(&self.height);
            let area = resolver.with_height(*height);

            layout.add_text(
                area,
                state.get(&self.text).as_ref(),
                *state.get(&self.font_size),
                *state.get(&self.color),
                *state.get(&self.horizontal_alignment),
                *state.get(&self.vertical_alignment),
            );
        }
    }
}

pub mod button {
    use std::marker::PhantomData;

    use rust_state::{Context, RustState, Selector};

    use crate::application::Appli;
    use crate::element::Element;
    use crate::element::id::ElementIdGenerator;
    use crate::element::store::ElementStore;
    use crate::event::ClickAction;
    use crate::layout::alignment::{HorizontalAlignment, VerticalAlignment};
    use crate::layout::{Layout, Resolver};
    use crate::theme::{ThemePathGetter, theme};

    #[derive(RustState)]
    pub struct ButtonTheme<App>
    where
        App: Appli + 'static,
    {
        pub foreground_color: App::Color,
        pub background_color: App::Color,
        pub hovered_foreground_color: App::Color,
        pub hovered_background_color: App::Color,
        pub height: f32,
        pub corner_radius: App::CornerRadius,
        pub font_size: App::FontSize,
        pub text_alignment: HorizontalAlignment,
        pub vertical_alignment: VerticalAlignment,
    }

    pub struct Button<Text, A, B, C, D, E, F, G, H, I, J, K> {
        pub text_marker: PhantomData<Text>,
        pub text: A,
        pub event: B,
        pub disabled: C,
        pub foreground_color: D,
        pub background_color: E,
        pub hovered_foreground_color: F,
        pub hovered_background_color: G,
        pub height: H,
        pub corner_radius: I,
        pub font_size: J,
        pub text_alignment: K,
    }

    impl<App, Text, A, B, C, D, E, F, G, H, I, J, K> Element<App> for Button<Text, A, B, C, D, E, F, G, H, I, J, K>
    where
        App: Appli,
        Text: AsRef<str> + 'static,
        A: Selector<App, Text>,
        B: ClickAction<App> + 'static,
        C: Selector<App, bool>,
        D: Selector<App, App::Color>,
        E: Selector<App, App::Color>,
        F: Selector<App, App::Color>,
        G: Selector<App, App::Color>,
        H: Selector<App, f32>,
        I: Selector<App, App::CornerRadius>,
        J: Selector<App, App::FontSize>,
        K: Selector<App, HorizontalAlignment>,
    {
        fn get_height(&self, state: &Context<App>, _: &ElementStore, _: &mut ElementIdGenerator, resolver: &mut Resolver) {
            let height = state.get(&self.height);
            resolver.with_height(*height);
        }

        fn create_layout<'a>(
            &'a self,
            state: &'a Context<App>,
            store: &'a ElementStore,
            _: &mut ElementIdGenerator,
            resolver: &mut Resolver,
            layout: &mut Layout<'a, App>,
        ) {
            let height = state.get(&self.height);
            let area = resolver.with_height(*height);
            let is_hoverered = layout.is_area_hovered_and_active(area);

            if is_hoverered {
                layout.add_click_area(area, &self.event);
                layout.mark_hovered();
            }

            layout.add_focus_area(area, store.get_element_id());

            let background_color = match is_hoverered {
                true => *state.get(&self.hovered_background_color),
                false => *state.get(&self.background_color),
            };

            // TODO: Temp
            if !layout.is_element_focused(store.get_element_id()) {
                layout.add_rectangle(area, *state.get(&self.corner_radius), background_color);
            }

            let foreground_color = match is_hoverered {
                true => *state.get(&self.hovered_foreground_color),
                false => *state.get(&self.foreground_color),
            };

            layout.add_text(
                area,
                state.get(&self.text).as_ref(),
                *state.get(&self.font_size),
                foreground_color,
                *state.get(&self.text_alignment),
                *state.get(&theme().button().vertical_alignment()),
            );
        }
    }
}

pub mod state_button {
    use std::marker::PhantomData;

    use rust_state::{Context, RustState, Selector};

    use crate::application::Appli;
    use crate::element::Element;
    use crate::element::id::ElementIdGenerator;
    use crate::element::store::ElementStore;
    use crate::event::ClickAction;
    use crate::layout::alignment::{HorizontalAlignment, VerticalAlignment};
    use crate::layout::area::Area;
    use crate::layout::{Layout, Resolver};
    use crate::theme::{ThemePathGetter, theme};

    #[derive(RustState)]
    pub struct StateButtonTheme<App>
    where
        App: Appli,
    {
        pub foreground_color: App::Color,
        pub background_color: App::Color,
        pub hovered_foreground_color: App::Color,
        pub hovered_background_color: App::Color,
        pub checkbox_color: App::Color,
        pub height: f32,
        pub corner_radius: App::CornerRadius,
        pub font_size: App::FontSize,
        pub text_alignment: HorizontalAlignment,
        pub vertical_alignment: VerticalAlignment,
    }

    pub struct StateButton<Text, A, B, C, D, E, F, G, H, I, J, K, L, M> {
        pub text_marker: PhantomData<Text>,
        pub text: A,
        pub state: B,
        pub event: C,
        pub disabled: D,
        pub foreground_color: E,
        pub background_color: F,
        pub hovered_foreground_color: G,
        pub hovered_background_color: H,
        pub checkbox_color: I,
        pub height: J,
        pub corner_radius: K,
        pub font_size: L,
        pub text_alignment: M,
    }

    impl<App, Text, A, B, C, D, E, F, G, H, I, J, K, L, M> Element<App> for StateButton<Text, A, B, C, D, E, F, G, H, I, J, K, L, M>
    where
        App: Appli,
        Text: AsRef<str> + 'static,
        A: Selector<App, Text>,
        B: Selector<App, bool>,
        C: ClickAction<App> + 'static,
        D: Selector<App, bool>,
        E: Selector<App, App::Color>,
        F: Selector<App, App::Color>,
        G: Selector<App, App::Color>,
        H: Selector<App, App::Color>,
        I: Selector<App, App::Color>,
        J: Selector<App, f32>,
        K: Selector<App, App::CornerRadius>,
        L: Selector<App, App::FontSize>,
        M: Selector<App, HorizontalAlignment>,
    {
        fn get_height(&self, state: &Context<App>, _: &ElementStore, _: &mut ElementIdGenerator, resolver: &mut Resolver) {
            let height = state.get(&self.height);
            resolver.with_height(*height);
        }

        fn create_layout<'a>(
            &'a self,
            state: &'a Context<App>,
            store: &'a ElementStore,
            _: &mut ElementIdGenerator,
            resolver: &mut Resolver,
            layout: &mut Layout<'a, App>,
        ) {
            let height = state.get(&self.height);
            let area = resolver.with_height(*height);
            let is_hoverered = layout.is_area_hovered_and_active(area);

            if is_hoverered {
                layout.add_click_area(area, &self.event);
                layout.mark_hovered();
            }

            layout.add_focus_area(area, store.get_element_id());

            let background_color = match is_hoverered {
                true => *state.get(&self.hovered_background_color),
                false => *state.get(&self.background_color),
            };

            layout.add_rectangle(area, *state.get(&self.corner_radius), background_color);

            let foreground_color = match is_hoverered {
                true => *state.get(&self.hovered_foreground_color),
                false => *state.get(&self.foreground_color),
            };

            layout.add_text(
                area,
                state.get(&self.text).as_ref(),
                *state.get(&self.font_size),
                foreground_color,
                *state.get(&self.text_alignment),
                *state.get(&theme().state_button().vertical_alignment()),
            );

            let checkbox_size = area.height - 6.0;
            layout.add_checkbox(
                Area {
                    x: area.x + 8.0,
                    y: area.y + 3.0,
                    width: checkbox_size,
                    height: checkbox_size,
                },
                *state.get(&self.checkbox_color),
                *state.get(&self.state),
            );
        }
    }
}

pub mod collapsable {
    use std::cell::RefCell;
    use std::marker::PhantomData;

    use rust_state::{Context, RustState, Selector};

    use crate::application::Appli;
    use crate::element::id::ElementIdGenerator;
    use crate::element::store::{ElementStore, Persistent, PersistentExt};
    use crate::element::{Element, ElementSet};
    use crate::layout::alignment::{HorizontalAlignment, VerticalAlignment};
    use crate::layout::area::Area;
    use crate::layout::{Layout, Resolver};
    use crate::theme::{ThemePathGetter, theme};

    #[derive(RustState)]
    pub struct CollapsableTheme<App>
    where
        App: Appli,
    {
        pub foreground_color: App::Color,
        pub hovered_foreground_color: App::Color,
        pub background_color: App::Color,
        pub gaps: f32,
        pub border: f32,
        pub corner_radius: App::CornerRadius,
        pub title_height: f32,
        pub font_size: App::FontSize,
        pub text_alignment: HorizontalAlignment,
        pub vertical_alignment: VerticalAlignment,
    }

    #[derive(Default)]
    pub struct CollapsableData {
        expanded: RefCell<bool>,
        // animation_state: AnimationState,
    }

    pub struct Collapsable<Text, A, B, C, D, E, F, G, H, I, J, Children> {
        pub text_marker: PhantomData<Text>,
        pub text: A,
        pub foreground_color: B,
        pub hovered_foreground_color: C,
        pub background_color: D,
        pub gaps: E,
        pub border: F,
        pub corner_radius: G,
        pub title_height: H,
        pub font_size: I,
        pub text_alignment: J,
        pub children: Children,
    }

    impl<Text, A, B, C, D, E, F, G, H, I, J, Children> Persistent for Collapsable<Text, A, B, C, D, E, F, G, H, I, J, Children> {
        type Data = CollapsableData;
    }

    impl<App, Text, A, B, C, D, E, F, G, H, I, J, Children> Element<App> for Collapsable<Text, A, B, C, D, E, F, G, H, I, J, Children>
    where
        App: Appli,
        Text: AsRef<str>,
        A: Selector<App, Text>,
        B: Selector<App, App::Color>,
        C: Selector<App, App::Color>,
        D: Selector<App, App::Color>,
        E: Selector<App, f32>,
        F: Selector<App, f32>,
        G: Selector<App, App::CornerRadius>,
        H: Selector<App, f32>,
        I: Selector<App, App::FontSize>,
        J: Selector<App, HorizontalAlignment>,
        Children: ElementSet<App>,
    {
        fn get_height(&self, state: &Context<App>, store: &ElementStore, generator: &mut ElementIdGenerator, resolver: &mut Resolver) {
            let persistent = self.get_persistent_data(store, ());

            let title_height = *state.get(&self.title_height);
            match *persistent.expanded.borrow() {
                true => resolver.with_derived(*state.get(&self.gaps), *state.get(&self.border), |resolver| {
                    resolver.push_top(title_height);
                    self.children.get_height(state, store, generator, resolver);
                }),
                false => resolver.with_height(title_height),
            };
        }

        fn create_layout<'a>(
            &'a self,
            state: &'a Context<App>,
            store: &'a ElementStore,
            generator: &mut ElementIdGenerator,
            resolver: &mut Resolver,
            layout: &mut Layout<'a, App>,
        ) {
            let persistent = self.get_persistent_data(store, ());

            let title_height = *state.get(&self.title_height);
            let area = match *persistent.expanded.borrow() {
                true => {
                    resolver.with_derived(*state.get(&self.gaps), *state.get(&self.border), |resolver| {
                        resolver.push_top(title_height);
                        // TODO: Very much temp
                        layout.push_layer();
                        self.children.create_layout(state, store, generator, resolver, layout);
                        // TODO: Very much temp
                        layout.pop_layer();
                    })
                }
                false => resolver.with_height(title_height),
            };

            let title_area = Area {
                x: area.x,
                y: area.y,
                width: area.width,
                height: title_height,
            };

            layout.add_rectangle(area, *state.get(&self.corner_radius), *state.get(&self.background_color));

            let is_title_hovered = layout.is_area_hovered_and_active(title_area);

            if is_title_hovered {
                layout.add_toggle(title_area, &persistent.expanded);
                layout.mark_hovered();
            }

            let foreground_color = match is_title_hovered {
                true => *state.get(&self.hovered_foreground_color),
                false => *state.get(&self.foreground_color),
            };

            layout.add_text(
                title_area,
                state.get(&self.text).as_ref(),
                *state.get(&self.font_size),
                foreground_color,
                *state.get(&self.text_alignment),
                *state.get(&theme().collapsable().vertical_alignment()),
            );
        }
    }
}

pub mod scroll_view {
    use std::cell::RefCell;

    use rust_state::Context;

    use crate::application::Appli;
    use crate::element::id::ElementIdGenerator;
    use crate::element::store::{ElementStore, Persistent, PersistentExt};
    use crate::element::{Element, ElementSet};
    use crate::layout::{HeightBound, Layout, Resolver};

    #[derive(Default)]
    pub struct PersistentData {
        scroll: RefCell<f32>,
        // animation_state: AnimationState,
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
        App: Appli,
        Children: ElementSet<App>,
    {
        fn get_height(&self, state: &Context<App>, store: &ElementStore, generator: &mut ElementIdGenerator, resolver: &mut Resolver) {
            resolver.with_derived_scrolled(0.0, self.height_bound, |resolver| {
                self.children.get_height(state, store, generator, resolver);
            });
        }

        fn create_layout<'a>(
            &'a self,
            state: &'a Context<App>,
            store: &'a ElementStore,
            generator: &mut ElementIdGenerator,
            resolver: &mut Resolver,
            layout: &mut Layout<'a, App>,
        ) {
            let persistent = self.get_persistent_data(store, ());

            if self.height_bound == HeightBound::Unbound {
                println!("unbound scroll views don't do anything");
            }

            let (height, content_height) = {
                let mut cloned_resolver = resolver.clone();

                let (children_area, content_height) = cloned_resolver.with_derived_scrolled(0.0, self.height_bound, |resolver| {
                    self.children.get_height(state, store, generator, resolver);
                });

                (children_area.height, content_height)
            };

            let max_scroll = (content_height - height).max(0.0);
            let final_scroll = {
                let mut current_scroll = persistent.scroll.borrow_mut();

                if *current_scroll > max_scroll {
                    *current_scroll = max_scroll;
                } else if *current_scroll < 0.0 {
                    *current_scroll = 0.0;
                }

                *current_scroll
            };

            let area = layout.with_clip_layer(|layout| {
                resolver
                    .with_derived_scrolled(final_scroll, self.height_bound, |resolver| {
                        // resolver.push_top(title_height);
                        // TODO: Very much temp
                        layout.push_layer();
                        self.children.create_layout(state, store, generator, resolver, layout);
                        // TODO: Very much temp
                        layout.pop_layer();
                    })
                    .0
            });

            if layout.is_area_hovered(area) {
                layout.add_scroll_area(area, max_scroll, &persistent.scroll);
            }
        }
    }
}

pub mod text_box {
    use std::marker::PhantomData;

    use rust_state::{Context, RustState, Selector};

    use crate::application::Appli;
    use crate::element::id::ElementIdGenerator;
    use crate::element::store::{ElementStore, Persistent, PersistentExt};
    use crate::element::{Element, ElementSet};
    use crate::layout::alignment::{HorizontalAlignment, VerticalAlignment};
    use crate::layout::{HeightBound, InputHandler, Layout, Resolver};
    use crate::theme::ThemePathGetter;

    #[derive(RustState)]
    pub struct TextBoxTheme<App>
    where
        App: Appli,
    {
        pub foreground_color: App::Color,
        pub background_color: App::Color,
        pub hovered_foreground_color: App::Color,
        pub hovered_background_color: App::Color,
        pub height: f32,
        pub corner_radius: App::CornerRadius,
        pub font_size: App::FontSize,
        pub text_alignment: HorizontalAlignment,
        pub vertical_alignment: VerticalAlignment,
    }

    pub struct TextBox<Text, A, B, C, D, E, F, G, H, I, J, K, L> {
        pub text_marker: PhantomData<Text>,
        pub text: A,
        pub state: B,
        pub input_handler: C,
        pub disabled: D,
        pub foreground_color: E,
        pub background_color: F,
        pub hovered_foreground_color: G,
        pub hovered_background_color: H,
        pub height: I,
        pub corner_radius: J,
        pub font_size: K,
        pub text_alignment: L,
    }

    impl<App, Text, A, B, C, D, E, F, G, H, I, J, K, L> Element<App> for TextBox<Text, A, B, C, D, E, F, G, H, I, J, K, L>
    where
        App: Appli,
        Text: AsRef<str> + 'static,
        A: Selector<App, Text>,
        B: Selector<App, String>,
        C: InputHandler<App> + 'static,
        D: Selector<App, bool>,
        E: Selector<App, App::Color>,
        F: Selector<App, App::Color>,
        G: Selector<App, App::Color>,
        H: Selector<App, App::Color>,
        I: Selector<App, f32>,
        J: Selector<App, App::CornerRadius>,
        K: Selector<App, App::FontSize>,
        L: Selector<App, HorizontalAlignment>,
    {
        fn get_height(&self, state: &Context<App>, _: &ElementStore, _: &mut ElementIdGenerator, resolver: &mut Resolver) {
            let height = state.get(&self.height);
            resolver.with_height(*height);
        }

        fn create_layout<'a>(
            &'a self,
            state: &'a Context<App>,
            store: &'a ElementStore,
            _: &mut ElementIdGenerator,
            resolver: &mut Resolver,
            layout: &mut Layout<'a, App>,
        ) {
            let height = state.get(&self.height);
            let area = resolver.with_height(*height);
            let is_hoverered = layout.is_area_hovered_and_active(area);

            if is_hoverered {
                layout.mark_hovered();
            }

            layout.add_focus_area(area, store.get_element_id());

            if layout.is_element_focused(store.get_element_id()) {
                layout.add_input_handler(&self.input_handler);
            }

            let background_color = match is_hoverered {
                true => *state.get(&self.hovered_background_color),
                false => *state.get(&self.background_color),
            };

            // TODO: Remove if
            if !layout.is_element_focused(store.get_element_id()) {
                layout.add_rectangle(area, *state.get(&self.corner_radius), background_color);
            }

            let foreground_color = match is_hoverered {
                true => *state.get(&self.hovered_foreground_color),
                false => *state.get(&self.foreground_color),
            };

            // layout.add_text(
            //     area,
            //     state.get(&self.text).as_ref(),
            //     *state.get(&self.font_size),
            //     foreground_color,
            //     *state.get(&self.text_alignment),
            //     *state.get(&theme().text_box().vertical_alignment()),
            // );

            layout.add_text(
                area,
                state.get(&self.state).as_str(),
                *state.get(&self.font_size),
                foreground_color,
                *state.get(&self.text_alignment),
                *state.get(&crate::theme::theme().text_box().vertical_alignment()),
            );
        }
    }

    pub struct DefaultHandler<P>(pub P);

    impl<App, P> InputHandler<App> for DefaultHandler<P>
    where
        P: rust_state::Path<App, String>,
    {
        fn handle_character(&self, state: &Context<App>, character: char) {
            if character == '\x08' {
                state.update_value_with(self.0, move |current_text| {
                    if !current_text.is_empty() {
                        current_text.pop();
                    }
                });
            } else {
                state.update_value_with(self.0, move |current_text| {
                    current_text.push(character);
                });
            }
        }
    }
}
