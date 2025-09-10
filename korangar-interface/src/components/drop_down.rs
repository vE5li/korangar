use std::cmp::Ordering;
use std::marker::PhantomData;
use std::num::NonZeroU32;

use interface_components::scroll_view;
use rust_state::{Context, Path, RustState, Selector};

use crate::application::{Application, Position, Size};
use crate::element::store::{ElementStore, ElementStoreMut};
use crate::element::{DefaultLayoutInfo, Element, ErasedElement};
use crate::event::{ClickHandler, Event, EventQueue};
use crate::layout::alignment::{HorizontalAlignment, VerticalAlignment};
use crate::layout::area::Area;
use crate::layout::{MouseButton, Resolver, WindowLayout};
use crate::theme::{ThemePathGetter, theme};

pub trait DropDownItem<T> {
    fn text(&self) -> &str;

    fn value(&self) -> T;
}

// TODO: Having this here is not very clean. We should instead have
// show_point_shadow_map wrapped in a new type and implement for that. Ideally,
// that would be an enum only covering the possible cases.
impl DropDownItem<Option<NonZeroU32>> for Option<NonZeroU32> {
    fn text(&self) -> &str {
        match self {
            Some(count) => match count.get() {
                1 => "1",
                2 => "2",
                3 => "3",
                4 => "4",
                5 => "5",
                6 => "6",
                _ => unimplemented!(),
            },
            None => "Off",
        }
    }

    fn value(&self) -> Option<NonZeroU32> {
        *self
    }
}

struct InnerButton<Value, Item, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O> {
    text_marker: PhantomData<(Value, Item)>,
    options: A,
    option_index: usize,
    event: B,
    foreground_color: C,
    background_color: D,
    highlight_color: E,
    hovered_foreground_color: F,
    hovered_background_color: G,
    shadow_color: H,
    shadow_padding: I,
    height: J,
    corner_diameter: K,
    font_size: L,
    horizontal_alignment: M,
    vertical_alignment: N,
    overflow_behavior: O,
}

impl<App, Value, Item, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O> Element<App>
    for InnerButton<Value, Item, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O>
where
    App: Application,
    Item: DropDownItem<Value>,
    A: Selector<App, Vec<Item>>,
    B: ClickHandler<App> + 'static,
    C: Selector<App, App::Color>,
    D: Selector<App, App::Color>,
    E: Selector<App, App::Color>,
    F: Selector<App, App::Color>,
    G: Selector<App, App::Color>,
    H: Selector<App, App::Color>,
    I: Selector<App, App::ShadowPadding>,
    J: Selector<App, f32>,
    K: Selector<App, App::CornerDiameter>,
    L: Selector<App, App::FontSize>,
    M: Selector<App, HorizontalAlignment>,
    N: Selector<App, VerticalAlignment>,
    O: Selector<App, App::OverflowBehavior>,
{
    fn create_layout_info(&mut self, state: &Context<App>, _: ElementStoreMut<'_>, resolver: &mut Resolver<'_, App>) -> Self::LayoutInfo {
        let height = *state.get(&self.height);
        let option = &state.get(&self.options)[self.option_index];

        let text = option.text();
        let font_size = *state.get(&self.font_size);
        let foreground_color = *state.get(&self.foreground_color);
        let highlight_color = *state.get(&self.highlight_color);
        let horizontal_alignment = *state.get(&self.horizontal_alignment);
        let overflow_behavior = *state.get(&self.overflow_behavior);

        let (size, font_size) = resolver.get_text_dimensions(
            text,
            foreground_color,
            highlight_color,
            font_size,
            horizontal_alignment,
            overflow_behavior,
        );

        let area = resolver.with_height(height.max(size.height()));

        Self::LayoutInfo { area, font_size }
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a Context<App>,
        _: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, App>,
    ) {
        let is_hoverered = layout_info.area.check().run(layout);

        if is_hoverered {
            layout.register_click_handler(MouseButton::Left, &self.event);
        }

        let background_color = match is_hoverered {
            true => *state.get(&self.hovered_background_color),
            false => *state.get(&self.background_color),
        };

        layout.add_rectangle(
            layout_info.area,
            *state.get(&self.corner_diameter),
            background_color,
            *state.get(&self.shadow_color),
            *state.get(&self.shadow_padding),
        );

        let foreground_color = match is_hoverered {
            true => *state.get(&self.hovered_foreground_color),
            false => *state.get(&self.foreground_color),
        };

        let option = &state.get(&self.options)[self.option_index];

        layout.add_text(
            layout_info.area,
            option.text(),
            layout_info.font_size,
            foreground_color,
            *state.get(&self.highlight_color),
            *state.get(&self.horizontal_alignment),
            *state.get(&self.vertical_alignment),
            *state.get(&self.overflow_behavior),
        );
    }
}

struct InnerElement<App, Value, Item, A, B>
where
    App: Application,
{
    value_path: A,
    options: B,
    item_boxes: Vec<Box<dyn Element<App, LayoutInfo = DefaultLayoutInfo<App>>>>,
    _marker: PhantomData<(App, Value, Item)>,
}

impl<App, Value, Item, A, B> Element<App> for InnerElement<App, Value, Item, A, B>
where
    App: Application,
    Value: 'static,
    Item: DropDownItem<Value> + 'static,
    A: Path<App, Value>,
    B: Selector<App, Vec<Item>> + Clone,
{
    // TODO: Refactor to not have to re-allocate this every frame.
    type LayoutInfo = (Area, Vec<DefaultLayoutInfo<App>>);

    fn create_layout_info(
        &mut self,
        state: &Context<App>,
        mut store: ElementStoreMut<'_>,
        resolver: &mut Resolver<'_, App>,
    ) -> Self::LayoutInfo {
        let vector = state.get(&self.options);

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
                        let options = self.options.clone();

                        let item_box: Box<dyn Element<App, LayoutInfo = DefaultLayoutInfo<App>>> = Box::new(InnerButton {
                            text_marker: PhantomData,
                            options: options.clone(),
                            option_index: index,
                            event: move |state: &Context<App>, queue: &mut EventQueue<App>| {
                                let options = state.get(&options);
                                let value = options[index].value();
                                state.update_value(value_path, value);
                                queue.queue(Event::CloseOverlay);
                            },
                            // TODO: These currently cannot be overwritten from the outside. This
                            // may be fine but also may be something that we want to change.
                            foreground_color: theme().drop_down().item_foreground_color(),
                            background_color: theme().drop_down().item_background_color(),
                            highlight_color: theme().drop_down().item_highlight_color(),
                            hovered_foreground_color: theme().drop_down().item_hovered_foreground_color(),
                            hovered_background_color: theme().drop_down().item_hovered_background_color(),
                            shadow_color: theme().drop_down().item_shadow_color(),
                            shadow_padding: theme().drop_down().item_shadow_padding(),
                            height: theme().drop_down().item_height(),
                            corner_diameter: theme().drop_down().item_corner_diameter(),
                            font_size: theme().drop_down().item_font_size(),
                            horizontal_alignment: theme().drop_down().item_horizontal_alignment(),
                            vertical_alignment: theme().drop_down().item_vertical_alignment(),
                            overflow_behavior: theme().drop_down().item_overflow_behavior(),
                        });
                        item_box
                    });
                }
            }
            Ordering::Equal => {}
        }

        let gaps = *state.get(&theme().drop_down().list_gaps());
        let border = *state.get(&theme().drop_down().list_border());

        let (area, layout_info) = resolver.with_derived(gaps, border, |resolver| {
            self.item_boxes
                .iter_mut()
                .enumerate()
                .map(|(index, item_box)| item_box.create_layout_info(state, store.child_store(index as u64), resolver))
                .collect()
        });

        (area, layout_info)
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a Context<App>,
        store: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, App>,
    ) {
        layout.add_rectangle(
            layout_info.0,
            *state.get(&theme().drop_down().list_corner_diameter()),
            *state.get(&theme().drop_down().list_background_color()),
            *state.get(&theme().drop_down().list_shadow_color()),
            *state.get(&theme().drop_down().list_shadow_padding()),
        );

        for (index, item_box) in self.item_boxes.iter().enumerate() {
            item_box.lay_out(state, store.child_store(index as u64), &layout_info.1[index], layout);
        }
    }
}

struct InnerClickHandler<App, Value, Item, A, B>
where
    App: Application,
{
    value_path: A,
    options: B,
    position: App::Position,
    size: App::Size,
    window_id: u64,
    _marker: PhantomData<(Value, Item)>,
}

impl<App, Value, Item, A, B> ClickHandler<App> for InnerClickHandler<App, Value, Item, A, B>
where
    App: Application,
    Value: 'static,
    Item: DropDownItem<Value> + 'static,
    A: Path<App, Value>,
    B: Selector<App, Vec<Item>> + Clone,
{
    fn handle_click(&self, _: &Context<App>, queue: &mut EventQueue<App>) {
        let element = ErasedElement::new(scroll_view! {
            children: (
                InnerElement {
                    value_path: self.value_path,
                    options: self.options.clone(),
                    item_boxes: Vec::new(),
                    _marker: PhantomData,
                },
            ),
        });

        queue.queue(Event::OpenOverlay {
            element,
            position: self.position,
            size: self.size,
            window_id: self.window_id,
        });
    }
}

struct DefaultClickHandler<App, Value, Item, A, B>
where
    App: Application,
{
    overlay_element: InnerClickHandler<App, Value, Item, A, B>,
    _marker: PhantomData<(App, Value, Item)>,
}

impl<App, Value, Item, A, B> DefaultClickHandler<App, Value, Item, A, B>
where
    App: Application,
    Value: 'static,
{
    pub fn new(value_path: A, options_path: B) -> DefaultClickHandler<App, Value, Item, A, B> {
        DefaultClickHandler {
            overlay_element: InnerClickHandler {
                value_path,
                options: options_path,
                position: App::Position::new(0.0, 0.0),
                size: App::Size::new(0.0, 0.0),
                window_id: 0,
                _marker: PhantomData,
            },
            _marker: PhantomData,
        }
    }

    fn set_position_size(&mut self, position: App::Position, size: App::Size, window_id: u64) {
        self.overlay_element.position = position;
        self.overlay_element.size = size;
        self.overlay_element.window_id = window_id;
    }
}

#[derive(RustState)]
pub struct DropDownTheme<App>
where
    App: Application + 'static,
{
    pub item_foreground_color: App::Color,
    pub item_background_color: App::Color,
    pub item_highlight_color: App::Color,
    pub item_hovered_foreground_color: App::Color,
    pub item_hovered_background_color: App::Color,
    pub item_shadow_color: App::Color,
    pub item_shadow_padding: App::ShadowPadding,
    pub item_height: f32,
    pub item_corner_diameter: App::CornerDiameter,
    pub item_font_size: App::FontSize,
    pub item_horizontal_alignment: HorizontalAlignment,
    pub item_vertical_alignment: VerticalAlignment,
    pub item_overflow_behavior: App::OverflowBehavior,
    pub list_corner_diameter: App::CornerDiameter,
    pub list_background_color: App::Color,
    pub list_shadow_color: App::Color,
    pub list_shadow_padding: App::ShadowPadding,
    pub list_gaps: f32,
    pub list_border: f32,
    pub list_maximum_height: f32,
    pub button_foreground_color: App::Color,
    pub button_background_color: App::Color,
    pub button_highlight_color: App::Color,
    pub button_hovered_foreground_color: App::Color,
    pub button_hovered_background_color: App::Color,
    pub button_shadow_color: App::Color,
    pub button_shadow_padding: App::ShadowPadding,
    pub button_height: f32,
    pub button_corner_diameter: App::CornerDiameter,
    pub button_font_size: App::FontSize,
    pub button_horizontal_alignment: HorizontalAlignment,
    pub button_vertical_alignment: VerticalAlignment,
    pub button_overflow_behavior: App::OverflowBehavior,
}

pub struct DropDown<App, Value, Item, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O>
where
    App: Application,
{
    options: A,
    selected: B,
    foreground_color: C,
    background_color: D,
    highlight_color: E,
    hovered_foreground_color: F,
    hovered_background_color: G,
    shadow_color: H,
    shadow_padding: I,
    height: J,
    corner_diameter: K,
    font_size: L,
    horizontal_alignment: M,
    vertical_alignment: N,
    overflow_behavior: O,
    click_handler: DefaultClickHandler<App, Value, Item, B, A>,
}

impl<App, Value, Item, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O> DropDown<App, Value, Item, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O>
where
    App: Application,
    Value: 'static,
    A: Clone,
    B: Copy,
{
    /// This function is supposed to be called from a component macro and not
    /// intended to be called manually.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub fn component_new(
        options: A,
        selected: B,
        foreground_color: C,
        background_color: D,
        highlight_color: E,
        hovered_foreground_color: F,
        hovered_background_color: G,
        shadow_color: H,
        shadow_padding: I,
        height: J,
        corner_diameter: K,
        font_size: L,
        horizontal_alignment: M,
        vertical_alignment: N,
        overflow_behavior: O,
    ) -> Self {
        Self {
            options: options.clone(),
            selected,
            foreground_color,
            background_color,
            highlight_color,
            hovered_foreground_color,
            hovered_background_color,
            shadow_color,
            shadow_padding,
            height,
            corner_diameter,
            font_size,
            horizontal_alignment,
            vertical_alignment,
            overflow_behavior,
            click_handler: DefaultClickHandler::new(selected, options),
        }
    }
}

impl<App, Value, Item, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O> Element<App>
    for DropDown<App, Value, Item, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O>
where
    App: Application,
    Value: PartialEq + 'static,
    Item: DropDownItem<Value> + 'static,
    A: Selector<App, Vec<Item>> + Clone,
    B: Path<App, Value>,
    C: Selector<App, App::Color>,
    D: Selector<App, App::Color>,
    E: Selector<App, App::Color>,
    F: Selector<App, App::Color>,
    G: Selector<App, App::Color>,
    H: Selector<App, App::Color>,
    I: Selector<App, App::ShadowPadding>,
    J: Selector<App, f32>,
    K: Selector<App, App::CornerDiameter>,
    L: Selector<App, App::FontSize>,
    M: Selector<App, HorizontalAlignment>,
    N: Selector<App, VerticalAlignment>,
    O: Selector<App, App::OverflowBehavior>,
{
    fn create_layout_info(
        &mut self,
        state: &Context<App>,
        store: ElementStoreMut<'_>,
        resolver: &mut Resolver<'_, App>,
    ) -> Self::LayoutInfo {
        let mut height = *state.get(&self.height);
        let mut font_size = *state.get(&self.font_size);
        let foreground_color = *state.get(&self.foreground_color);
        let highlight_color = *state.get(&self.highlight_color);

        let selected = state.get(&self.selected);
        if let Some(index) = state.get(&self.options).iter().position(|value| value.value() == *selected)
            && let Some(selected_option) = state.get(&self.options).get(index)
        {
            let text = selected_option.text();
            let horizontal_alignment = *state.get(&self.horizontal_alignment);
            let overflow_behavior = *state.get(&self.overflow_behavior);

            let (size, new_font_size) = resolver.get_text_dimensions(
                text,
                foreground_color,
                highlight_color,
                font_size,
                horizontal_alignment,
                overflow_behavior,
            );

            height = height.max(size.height());
            font_size = new_font_size;
        };

        let area = resolver.with_height(height);

        let list_maximum_height = *state.get(&theme().drop_down().list_maximum_height());
        let border = *state.get(&theme().drop_down().list_border());

        self.click_handler.set_position_size(
            App::Position::new(area.left - border, area.top - border),
            App::Size::new(area.width + border * 2.0, list_maximum_height + border * 2.0),
            store.get_window_id(),
        );

        Self::LayoutInfo { area, font_size }
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a Context<App>,
        _: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, App>,
    ) {
        let is_hoverered = layout_info.area.check().run(layout);

        if is_hoverered {
            layout.register_click_handler(MouseButton::Left, &self.click_handler.overlay_element);
        }

        let background_color = match is_hoverered {
            true => *state.get(&self.hovered_background_color),
            false => *state.get(&self.background_color),
        };

        layout.add_rectangle(
            layout_info.area,
            *state.get(&self.corner_diameter),
            background_color,
            *state.get(&self.shadow_color),
            *state.get(&self.shadow_padding),
        );

        let foreground_color = match is_hoverered {
            true => *state.get(&self.hovered_foreground_color),
            false => *state.get(&self.foreground_color),
        };

        let selected = state.get(&self.selected);
        let Some(index) = state.get(&self.options).iter().position(|value| value.value() == *selected) else {
            return;
        };

        let Some(selected_option) = state.get(&self.options).get(index) else {
            return;
        };

        layout.add_text(
            layout_info.area,
            selected_option.text(),
            layout_info.font_size,
            foreground_color,
            *state.get(&self.highlight_color),
            *state.get(&self.horizontal_alignment),
            *state.get(&self.vertical_alignment),
            *state.get(&self.overflow_behavior),
        );
    }
}
