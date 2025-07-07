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
        fn make_layout(
            &mut self,
            state: &Context<App>,
            _: &mut ElementStore,
            _: &mut ElementIdGenerator,
            resolver: &mut Resolver,
        ) -> Self::Layouted {
            let height = state.get(&self.height);
            let area = resolver.with_height(*height);
            Self::Layouted { area }
        }

        fn create_layout<'a>(
            &'a self,
            state: &'a Context<App>,
            _: &'a ElementStore,
            layouted: &'a Self::Layouted,
            layout: &mut Layout<'a, App>,
        ) {
            layout.add_text(
                layouted.area,
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
        fn make_layout(
            &mut self,
            state: &Context<App>,
            store: &mut ElementStore,
            generator: &mut ElementIdGenerator,
            resolver: &mut Resolver,
        ) -> Self::Layouted {
            let height = state.get(&self.height);
            let area = resolver.with_height(*height);
            Self::Layouted { area }
        }

        fn create_layout<'a>(
            &'a self,
            state: &'a Context<App>,
            store: &'a ElementStore,
            layouted: &'a Self::Layouted,
            layout: &mut Layout<'a, App>,
        ) {
            let is_hoverered = layout.is_area_hovered_and_active(layouted.area);

            if is_hoverered {
                layout.add_click_area(layouted.area, &self.event);
                layout.mark_hovered();
            }

            layout.add_focus_area(layouted.area, store.get_element_id());

            let background_color = match is_hoverered {
                true => *state.get(&self.hovered_background_color),
                false => *state.get(&self.background_color),
            };

            // TODO: Temp
            if !layout.is_element_focused(store.get_element_id()) {
                layout.add_rectangle(layouted.area, *state.get(&self.corner_radius), background_color);
            }

            let foreground_color = match is_hoverered {
                true => *state.get(&self.hovered_foreground_color),
                false => *state.get(&self.foreground_color),
            };

            layout.add_text(
                layouted.area,
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
        fn make_layout(
            &mut self,
            state: &Context<App>,
            store: &mut ElementStore,
            generator: &mut ElementIdGenerator,
            resolver: &mut Resolver,
        ) -> Self::Layouted {
            let height = state.get(&self.height);
            let area = resolver.with_height(*height);
            Self::Layouted { area }
        }

        fn create_layout<'a>(
            &'a self,
            state: &'a Context<App>,
            store: &'a ElementStore,
            layouted: &'a Self::Layouted,
            layout: &mut Layout<'a, App>,
        ) {
            let is_hoverered = layout.is_area_hovered_and_active(layouted.area);

            if is_hoverered {
                layout.add_click_area(layouted.area, &self.event);
                layout.mark_hovered();
            }

            layout.add_focus_area(layouted.area, store.get_element_id());

            let background_color = match is_hoverered {
                true => *state.get(&self.hovered_background_color),
                false => *state.get(&self.background_color),
            };

            layout.add_rectangle(layouted.area, *state.get(&self.corner_radius), background_color);

            let foreground_color = match is_hoverered {
                true => *state.get(&self.hovered_foreground_color),
                false => *state.get(&self.foreground_color),
            };

            layout.add_text(
                layouted.area,
                state.get(&self.text).as_ref(),
                *state.get(&self.font_size),
                foreground_color,
                *state.get(&self.text_alignment),
                *state.get(&theme().state_button().vertical_alignment()),
            );

            let checkbox_size = layouted.area.height - 6.0;
            layout.add_checkbox(
                Area {
                    x: layouted.area.x + 8.0,
                    y: layouted.area.y + 3.0,
                    width: checkbox_size,
                    height: checkbox_size,
                },
                *state.get(&self.checkbox_color),
                *state.get(&self.state),
            );
        }
    }
}

pub mod drop_down {
    use std::cmp::Ordering;
    use std::marker::PhantomData;

    use rust_state::{Context, ManuallyAssertExt, Path, RustState, Selector, VecIndexExt};

    use crate::application::Appli;
    use crate::element::id::ElementIdGenerator;
    use crate::element::store::ElementStore;
    use crate::element::{DefaultLayouted, Element, ErasedElement};
    use crate::event::{ClickAction, Event, EventQueue};
    use crate::layout::alignment::{HorizontalAlignment, VerticalAlignment};
    use crate::layout::area::Area;
    use crate::layout::{Layout, Resolver};
    use crate::prelude::ButtonThemePathExt;
    use crate::theme::{ThemePathGetter, theme};

    pub trait DropDownItem<T> {
        fn text(&self) -> &str;

        fn value(&self) -> T;
    }

    // TODO: Pretty this up
    pub struct DefaultClickHandler<App, Value, Item, F> {
        overlay_element: F,
        _marker: PhantomData<(App, Value, Item)>,
    }

    impl<App, Value, Item, F> DefaultClickHandler<App, Value, Item, F>
    where
        App: Appli,
        Value: 'static,
        Item: DropDownItem<Value> + 'static,
    {
        pub fn new(
            value_path: impl Path<App, Value>,
            options_path: impl Path<App, Vec<Item>>,
        ) -> DefaultClickHandler<App, Value, Item, impl ClickAction<App>> {
            struct InnerButton<Value, Item, A, B, C, D, E, F, G, H, I, J> {
                pub text_marker: PhantomData<(Value, Item)>,
                pub option: A,
                pub event: B,
                pub foreground_color: C,
                pub background_color: D,
                pub hovered_foreground_color: E,
                pub hovered_background_color: F,
                pub height: G,
                pub corner_radius: H,
                pub font_size: I,
                pub text_alignment: J,
            }

            impl<App, Value, Item, A, B, C, D, E, F, G, H, I, J> Element<App> for InnerButton<Value, Item, A, B, C, D, E, F, G, H, I, J>
            where
                App: Appli,
                Item: DropDownItem<Value>,
                A: Selector<App, Item>,
                B: ClickAction<App> + 'static,
                C: Selector<App, App::Color>,
                D: Selector<App, App::Color>,
                E: Selector<App, App::Color>,
                F: Selector<App, App::Color>,
                G: Selector<App, f32>,
                H: Selector<App, App::CornerRadius>,
                I: Selector<App, App::FontSize>,
                J: Selector<App, HorizontalAlignment>,
            {
                fn make_layout(
                    &mut self,
                    state: &Context<App>,
                    _: &mut ElementStore,
                    _: &mut ElementIdGenerator,
                    resolver: &mut Resolver,
                ) -> Self::Layouted {
                    let height = state.get(&self.height);
                    let area = resolver.with_height(*height);
                    Self::Layouted { area }
                }

                fn create_layout<'a>(
                    &'a self,
                    state: &'a Context<App>,
                    store: &'a ElementStore,
                    layouted: &'a Self::Layouted,
                    layout: &mut Layout<'a, App>,
                ) {
                    let is_hoverered = layout.is_area_hovered_and_active(layouted.area);

                    if is_hoverered {
                        layout.add_click_area(layouted.area, &self.event);
                        layout.mark_hovered();
                    }

                    layout.add_focus_area(layouted.area, store.get_element_id());

                    let background_color = match is_hoverered {
                        true => *state.get(&self.hovered_background_color),
                        false => *state.get(&self.background_color),
                    };

                    layout.add_rectangle(layouted.area, *state.get(&self.corner_radius), background_color);

                    let foreground_color = match is_hoverered {
                        true => *state.get(&self.hovered_foreground_color),
                        false => *state.get(&self.foreground_color),
                    };

                    let option = state.get(&self.option);

                    layout.add_text(
                        layouted.area,
                        option.text(),
                        *state.get(&self.font_size),
                        foreground_color,
                        *state.get(&self.text_alignment),
                        *state.get(&theme().button().vertical_alignment()),
                    );
                }
            }

            struct InnerElement<App, Value, Item, A, B>
            where
                App: Appli,
            {
                value_path: A,
                options_path: B,
                item_boxes: Vec<Box<dyn Element<App, Layouted = DefaultLayouted>>>,
                _marker: PhantomData<(App, Value, Item)>,
            }

            impl<App, Value, Item, A, B> Element<App> for InnerElement<App, Value, Item, A, B>
            where
                App: Appli,
                Value: 'static,
                Item: DropDownItem<Value> + 'static,
                A: Path<App, Value>,
                B: Path<App, Vec<Item>>,
            {
                // TODO: Refactor to not have to re-allocate this every frame.
                type Layouted = (Area, Vec<DefaultLayouted>);

                fn make_layout(
                    &mut self,
                    state: &Context<App>,
                    store: &mut ElementStore,
                    generator: &mut ElementIdGenerator,
                    resolver: &mut Resolver,
                ) -> Self::Layouted {
                    let vector = state.get(&self.options_path);

                    match self.item_boxes.len().cmp(&vector.len()) {
                        Ordering::Greater => {
                            // Delete excess elements.
                            self.item_boxes.truncate(vector.len());
                        }
                        Ordering::Less => {
                            // Add new elements.
                            for index in self.item_boxes.len()..vector.len() {
                                self.item_boxes.push({
                                    let value_path = self.value_path;
                                    let option_path = self.options_path.index(index).manually_asserted();

                                    let item_box: Box<dyn Element<App, Layouted = DefaultLayouted>> = Box::new(InnerButton {
                                        text_marker: PhantomData,
                                        option: option_path,
                                        event: move |state: &Context<App>, queue: &mut EventQueue<App>| {
                                            let value = state.get(&option_path).value();
                                            state.update_value(value_path, value);
                                            queue.queue(Event::CloseOverlay);
                                        },
                                        // FIX: Don't use button theme
                                        foreground_color: theme().button().foreground_color(),
                                        background_color: theme().button().background_color(),
                                        hovered_foreground_color: theme().button().hovered_foreground_color(),
                                        hovered_background_color: theme().button().hovered_background_color(),
                                        height: theme().button().height(),
                                        corner_radius: theme().button().corner_radius(),
                                        font_size: theme().button().font_size(),
                                        text_alignment: theme().button().text_alignment(),
                                    });
                                    item_box
                                });
                            }
                        }
                        Ordering::Equal => {}
                    }

                    resolver.with_derived(2.0, 4.0, |resolver| {
                        self.item_boxes
                            .iter_mut()
                            .enumerate()
                            .map(|(index, item_box)| {
                                item_box.make_layout(
                                    state,
                                    store.get_or_create_child_store(index as u64, generator),
                                    generator,
                                    resolver,
                                )
                            })
                            .collect()
                    })
                }

                fn create_layout<'a>(
                    &'a self,
                    state: &'a Context<App>,
                    store: &'a ElementStore,
                    layouted: &'a Self::Layouted,
                    layout: &mut Layout<'a, App>,
                ) {
                    // TODO: Render the background.

                    for (index, item_box) in self.item_boxes.iter().enumerate() {
                        item_box.create_layout(state, store.child_store(index as u64), &layouted.1[index], layout);
                    }
                }
            }

            struct InnerClickAction<Value, Item, A, B> {
                value_path: A,
                options_path: B,
                _marker: PhantomData<(Value, Item)>,
            }

            impl<App, Value, Item, A, B> ClickAction<App> for InnerClickAction<Value, Item, A, B>
            where
                App: Appli,
                Value: 'static,
                Item: DropDownItem<Value> + 'static,
                A: Path<App, Value>,
                B: Path<App, Vec<Item>>,
            {
                fn execute(&self, _: &Context<App>, queue: &mut EventQueue<App>) {
                    let erased_element = ErasedElement::new(InnerElement {
                        value_path: self.value_path,
                        options_path: self.options_path,
                        item_boxes: Vec::new(),
                        _marker: PhantomData,
                    });

                    queue.queue(Event::OpenOverlay(Box::new(erased_element)));
                }
            }

            DefaultClickHandler {
                overlay_element: InnerClickAction {
                    value_path,
                    options_path,
                    _marker: PhantomData,
                },
                _marker: PhantomData,
            }
        }
    }

    #[derive(RustState)]
    pub struct DropDownTheme<App>
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

    pub struct DropDown<App, Value, Item, A, B, C, D, E, F, G, H, I, J, K> {
        pub options: A,
        pub selected: B,
        pub foreground_color: C,
        pub background_color: D,
        pub hovered_foreground_color: E,
        pub hovered_background_color: F,
        pub height: G,
        pub corner_radius: H,
        pub font_size: I,
        pub text_alignment: J,
        pub click_handler: DefaultClickHandler<App, Value, Item, K>,
    }

    impl<App, Value, Item, A, B, C, D, E, F, G, H, I, J, K> Element<App> for DropDown<App, Value, Item, A, B, C, D, E, F, G, H, I, J, K>
    where
        App: Appli,
        Value: PartialEq + 'static,
        Item: DropDownItem<Value>,
        A: Selector<App, Vec<Item>>,
        B: Selector<App, Value>,
        C: Selector<App, App::Color>,
        D: Selector<App, App::Color>,
        E: Selector<App, App::Color>,
        F: Selector<App, App::Color>,
        G: Selector<App, f32>,
        H: Selector<App, App::CornerRadius>,
        I: Selector<App, App::FontSize>,
        J: Selector<App, HorizontalAlignment>,
        K: ClickAction<App>,
    {
        fn make_layout(
            &mut self,
            state: &Context<App>,
            _: &mut ElementStore,
            _: &mut ElementIdGenerator,
            resolver: &mut Resolver,
        ) -> Self::Layouted {
            let height = state.get(&self.height);
            let area = resolver.with_height(*height);
            Self::Layouted { area }
        }

        fn create_layout<'a>(
            &'a self,
            state: &'a Context<App>,
            store: &'a ElementStore,
            layouted: &'a Self::Layouted,
            layout: &mut Layout<'a, App>,
        ) {
            let is_hoverered = layout.is_area_hovered_and_active(layouted.area);

            if is_hoverered {
                layout.add_click_area(layouted.area, &self.click_handler.overlay_element);
                layout.mark_hovered();
            }

            layout.add_focus_area(layouted.area, store.get_element_id());

            let background_color = match is_hoverered {
                true => *state.get(&self.hovered_background_color),
                false => *state.get(&self.background_color),
            };

            layout.add_rectangle(layouted.area, *state.get(&self.corner_radius), background_color);

            let foreground_color = match is_hoverered {
                true => *state.get(&self.hovered_foreground_color),
                false => *state.get(&self.foreground_color),
            };

            let selected = state.get(&self.selected);
            // TODO: Maybe don't unwrap here.
            let index = state
                .get(&self.options)
                .iter()
                .position(|value| value.value() == *selected)
                .unwrap();
            // TODO: Maybe don't unwrap here either.
            let selected_option = state.get(&self.options).get(index).unwrap();

            layout.add_text(
                layouted.area,
                selected_option.text(),
                *state.get(&self.font_size),
                foreground_color,
                *state.get(&self.text_alignment),
                // FIX: Don't use button theme
                *state.get(&theme().button().vertical_alignment()),
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
    use crate::element::store::{ElementStore, Persistent, PersistentData, PersistentExt};
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

    pub struct CollapsableData {
        expanded: RefCell<bool>,
    }

    impl PersistentData for CollapsableData {
        type Inputs = bool;

        fn new(inputs: Self::Inputs) -> Self {
            Self {
                expanded: RefCell::new(inputs),
            }
        }
    }

    pub struct Collapsable<Text, A, B, C, D, E, F, G, H, I, J, K, Children> {
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
        pub initially_expanded: K,
        pub children: Children,
    }

    impl<Text, A, B, C, D, E, F, G, H, I, J, K, Children> Persistent for Collapsable<Text, A, B, C, D, E, F, G, H, I, J, K, Children> {
        type Data = CollapsableData;
    }

    pub struct MyLayouted<C> {
        area: Area,
        children: Option<C>,
    }

    impl<App, Text, A, B, C, D, E, F, G, H, I, J, K, Children> Element<App> for Collapsable<Text, A, B, C, D, E, F, G, H, I, J, K, Children>
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
        K: Selector<App, bool>,
        Children: ElementSet<App>,
    {
        type Layouted = MyLayouted<Children::Layouted>;

        fn make_layout(
            &mut self,
            state: &Context<App>,
            store: &mut ElementStore,
            generator: &mut ElementIdGenerator,
            resolver: &mut Resolver,
        ) -> Self::Layouted {
            let persistent = self.get_persistent_data(store, *state.get(&self.initially_expanded));
            let expanded = *persistent.expanded.borrow();

            let title_height = *state.get(&self.title_height);

            let (area, children) = match expanded {
                true => resolver.with_derived(*state.get(&self.gaps), *state.get(&self.border), |resolver| {
                    resolver.push_top(title_height);
                    Some(self.children.make_layout(state, store, generator, resolver))
                }),
                false => (resolver.with_height(title_height), None),
            };

            MyLayouted { area, children }
        }

        fn create_layout<'a>(
            &'a self,
            state: &'a Context<App>,
            store: &'a ElementStore,
            layouted: &'a Self::Layouted,
            layout: &mut Layout<'a, App>,
        ) {
            if let Some(layouted) = &layouted.children {
                self.children.create_layout(state, store, layouted, layout);
            }

            let title_height = *state.get(&self.title_height);

            let title_area = Area {
                x: layouted.area.x,
                y: layouted.area.y,
                width: layouted.area.width,
                height: title_height,
            };

            layout.add_rectangle(
                layouted.area,
                *state.get(&self.corner_radius),
                *state.get(&self.background_color),
            );

            let is_title_hovered = layout.is_area_hovered_and_active(title_area);

            if is_title_hovered {
                let persistent = self.get_persistent_data(store, *state.get(&self.initially_expanded));
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

pub mod split {
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
}

pub mod scroll_view {
    use std::cell::RefCell;

    use rust_state::Context;

    use crate::application::Appli;
    use crate::element::id::ElementIdGenerator;
    use crate::element::store::{ElementStore, Persistent, PersistentExt};
    use crate::element::{DefaultLayoutedSet, Element, ElementSet};
    use crate::layout::area::Area;
    use crate::layout::{HeightBound, Layout, Resolver};

    #[derive(Default)]
    pub struct PersistentData {
        scroll: RefCell<f32>,
        // animation_state: AnimationState,
    }

    pub struct ScrollViewLayouted<L> {
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
        App: Appli,
        Children: ElementSet<App>,
    {
        type Layouted = ScrollViewLayouted<Children::Layouted>;

        fn make_layout(
            &mut self,
            state: &Context<App>,
            store: &mut ElementStore,
            generator: &mut ElementIdGenerator,
            resolver: &mut Resolver,
        ) -> Self::Layouted {
            loop {
                let persistent = self.get_persistent_data(store, ());
                let current_scroll = *persistent.scroll.borrow();

                // In case that we need to resolve twice we don't want to start with the same
                // resolver state as the first iteration, so we clone it here and assing it back
                // as soon as a correct layout was found. This is a little bit
                // ugly and might be improved in the future.
                let mut cloned_resolver = resolver.clone();

                let (area, children_height, layouted) =
                    cloned_resolver.with_derived_scrolled(current_scroll, self.height_bound, |resolver| {
                        self.children.make_layout(state, store, generator, resolver)
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

                return ScrollViewLayouted {
                    area,
                    children: layouted,
                    max_scroll,
                };
            }
        }

        fn create_layout<'a>(
            &'a self,
            state: &'a Context<App>,
            store: &'a ElementStore,
            layouted: &'a Self::Layouted,
            layout: &mut Layout<'a, App>,
        ) {
            let persistent = self.get_persistent_data(store, ());

            if self.height_bound == HeightBound::Unbound {
                println!("unbound scroll views don't do anything");
            }

            layout.with_clip_layer(layouted.area, |layout| {
                // TODO: Very much temp
                layout.push_layer();

                self.children.create_layout(state, store, &layouted.children, layout);

                // TODO: Very much temp
                layout.pop_layer();
            });

            if layout.is_area_hovered(layouted.area) {
                layout.add_scroll_area(layouted.area, layouted.max_scroll, &persistent.scroll);
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
        fn make_layout(
            &mut self,
            state: &Context<App>,
            store: &mut ElementStore,
            generator: &mut ElementIdGenerator,
            resolver: &mut Resolver,
        ) -> Self::Layouted {
            let height = state.get(&self.height);

            Self::Layouted {
                area: resolver.with_height(*height),
            }
        }

        fn create_layout<'a>(
            &'a self,
            state: &'a Context<App>,
            store: &'a ElementStore,
            layouted: &'a Self::Layouted,
            layout: &mut Layout<'a, App>,
        ) {
            let is_hoverered = layout.is_area_hovered_and_active(layouted.area);

            if is_hoverered {
                layout.mark_hovered();
            }

            layout.add_focus_area(layouted.area, store.get_element_id());

            if layout.is_element_focused(store.get_element_id()) {
                layout.add_input_handler(&self.input_handler);
            }

            let background_color = match is_hoverered {
                true => *state.get(&self.hovered_background_color),
                false => *state.get(&self.background_color),
            };

            // TODO: Remove if
            if !layout.is_element_focused(store.get_element_id()) {
                layout.add_rectangle(layouted.area, *state.get(&self.corner_radius), background_color);
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
                layouted.area,
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
