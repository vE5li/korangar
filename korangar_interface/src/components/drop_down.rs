use std::cmp::Ordering;
use std::marker::PhantomData;

use interface_components::scroll_view;
use rust_state::{Context, ManuallyAssertExt, Path, RustState, Selector, VecIndexExt};

use crate::application::{Application, PositionTrait, SizeTrait};
use crate::element::id::ElementIdGenerator;
use crate::element::store::ElementStore;
use crate::element::{DefaultLayoutInfo, Element, ErasedElement};
use crate::event::{ClickAction, Event, EventQueue};
use crate::layout::alignment::{HorizontalAlignment, VerticalAlignment};
use crate::layout::area::Area;
use crate::layout::{Layout, MouseButton, Resolver};
use crate::theme::{ThemePathGetter, theme};

pub trait DropDownItem<T> {
    fn text(&self) -> &str;

    fn value(&self) -> T;
}

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
    App: Application,
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
    fn create_layout_info(
        &mut self,
        state: &Context<App>,
        _: &mut ElementStore,
        _: &mut ElementIdGenerator,
        resolver: &mut Resolver,
    ) -> Self::LayoutInfo {
        let height = state.get(&self.height);
        let area = resolver.with_height(*height);
        Self::LayoutInfo { area }
    }

    fn layout_element<'a>(
        &'a self,
        state: &'a Context<App>,
        _: &'a ElementStore,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut Layout<'a, App>,
    ) {
        let is_hoverered = layout.is_area_hovered_and_active(layout_info.area);

        if is_hoverered {
            layout.add_click_area(layout_info.area, MouseButton::Left, &self.event);
            layout.mark_hovered();
        }

        let background_color = match is_hoverered {
            true => *state.get(&self.hovered_background_color),
            false => *state.get(&self.background_color),
        };

        layout.add_rectangle(layout_info.area, *state.get(&self.corner_radius), background_color);

        let foreground_color = match is_hoverered {
            true => *state.get(&self.hovered_foreground_color),
            false => *state.get(&self.foreground_color),
        };

        let option = state.get(&self.option);

        layout.add_text(
            layout_info.area,
            option.text(),
            *state.get(&self.font_size),
            foreground_color,
            *state.get(&self.text_alignment),
            *state.get(&theme().drop_down().item_vertical_alignment()),
        );
    }
}

struct InnerElement<App, Value, Item, A, B>
where
    App: Application,
{
    value_path: A,
    options_path: B,
    item_boxes: Vec<Box<dyn Element<App, LayoutInfo = DefaultLayoutInfo>>>,
    _marker: PhantomData<(App, Value, Item)>,
}

impl<App, Value, Item, A, B> Element<App> for InnerElement<App, Value, Item, A, B>
where
    App: Application,
    Value: 'static,
    Item: DropDownItem<Value> + 'static,
    A: Path<App, Value>,
    B: Path<App, Vec<Item>>,
{
    // TODO: Refactor to not have to re-allocate this every frame.
    type LayoutInfo = (Area, Vec<DefaultLayoutInfo>);

    fn create_layout_info(
        &mut self,
        state: &Context<App>,
        store: &mut ElementStore,
        generator: &mut ElementIdGenerator,
        resolver: &mut Resolver,
    ) -> Self::LayoutInfo {
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

                        let item_box: Box<dyn Element<App, LayoutInfo = DefaultLayoutInfo>> = Box::new(InnerButton {
                            text_marker: PhantomData,
                            option: option_path,
                            event: move |state: &Context<App>, queue: &mut EventQueue<App>| {
                                let value = state.get(&option_path).value();
                                state.update_value(value_path, value);
                                queue.queue(Event::CloseOverlay);
                            },
                            // TODO: These currently cannot be overwritten from the outside. This
                            // may be fine but also may be something that we want to change.
                            foreground_color: theme().drop_down().item_foreground_color(),
                            background_color: theme().drop_down().item_background_color(),
                            hovered_foreground_color: theme().drop_down().item_hovered_foreground_color(),
                            hovered_background_color: theme().drop_down().item_hovered_background_color(),
                            height: theme().drop_down().item_height(),
                            corner_radius: theme().drop_down().item_corner_radius(),
                            font_size: theme().drop_down().item_font_size(),
                            text_alignment: theme().drop_down().item_text_alignment(),
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
                .map(|(index, item_box)| {
                    item_box.create_layout_info(
                        state,
                        store.get_or_create_child_store(index as u64, generator),
                        generator,
                        resolver,
                    )
                })
                .collect()
        });

        (area, layout_info)
    }

    fn layout_element<'a>(
        &'a self,
        state: &'a Context<App>,
        store: &'a ElementStore,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut Layout<'a, App>,
    ) {
        layout.add_rectangle(
            layout_info.0,
            *state.get(&theme().drop_down().list_corner_radius()),
            *state.get(&theme().drop_down().list_background_color()),
        );

        for (index, item_box) in self.item_boxes.iter().enumerate() {
            item_box.layout_element(state, store.child_store(index as u64), &layout_info.1[index], layout);
        }
    }
}

struct InnerClickAction<App, Value, Item, A, B>
where
    App: Application,
{
    value_path: A,
    options_path: B,
    position: App::Position,
    size: App::Size,
    _marker: PhantomData<(Value, Item)>,
}

impl<App, Value, Item, A, B> ClickAction<App> for InnerClickAction<App, Value, Item, A, B>
where
    App: Application,
    Value: 'static,
    Item: DropDownItem<Value> + 'static,
    A: Path<App, Value>,
    B: Path<App, Vec<Item>>,
{
    fn execute(&self, _: &Context<App>, queue: &mut EventQueue<App>) {
        let erased_element = ErasedElement::new(scroll_view! {
            children: (
                InnerElement {
                    value_path: self.value_path,
                    options_path: self.options_path,
                    item_boxes: Vec::new(),
                    _marker: PhantomData,
                },
            ),
        });

        queue.queue(Event::OpenOverlay {
            element: Box::new(erased_element),
            position: self.position,
            size: self.size,
        });
    }
}

// TODO: Pretty this up
pub struct DefaultClickHandler<App, Value, Item, A, B>
where
    App: Application,
{
    overlay_element: InnerClickAction<App, Value, Item, A, B>,
    _marker: PhantomData<(App, Value, Item)>,
}

impl<App, Value, Item, A, B> DefaultClickHandler<App, Value, Item, A, B>
where
    App: Application,
    Value: 'static,
    Item: DropDownItem<Value> + 'static,
    A: Path<App, Value>,
    B: Path<App, Vec<Item>>,
{
    pub fn new(value_path: A, options_path: B) -> DefaultClickHandler<App, Value, Item, A, B> {
        DefaultClickHandler {
            overlay_element: InnerClickAction {
                value_path,
                options_path,
                position: App::Position::new(0.0, 0.0),
                size: App::Size::new(0.0, 0.0),
                _marker: PhantomData,
            },
            _marker: PhantomData,
        }
    }

    fn set_position_size(&mut self, position: App::Position, size: App::Size) {
        self.overlay_element.position = position;
        self.overlay_element.size = size;
    }
}

#[derive(RustState)]
pub struct DropDownTheme<App>
where
    App: Application + 'static,
{
    pub item_foreground_color: App::Color,
    pub item_background_color: App::Color,
    pub item_hovered_foreground_color: App::Color,
    pub item_hovered_background_color: App::Color,
    pub item_height: f32,
    pub item_corner_radius: App::CornerRadius,
    pub item_font_size: App::FontSize,
    pub item_text_alignment: HorizontalAlignment,
    pub item_vertical_alignment: VerticalAlignment,
    pub list_corner_radius: App::CornerRadius,
    pub list_background_color: App::Color,
    pub list_gaps: f32,
    pub list_border: f32,
    pub list_maximum_height: f32,
    pub button_foreground_color: App::Color,
    pub button_background_color: App::Color,
    pub button_hovered_foreground_color: App::Color,
    pub button_hovered_background_color: App::Color,
    pub button_height: f32,
    pub button_corner_radius: App::CornerRadius,
    pub button_font_size: App::FontSize,
    pub button_text_alignment: HorizontalAlignment,
    pub button_vertical_alignment: VerticalAlignment,
}

pub struct DropDown<App, Value, Item, A, B, C, D, E, F, G, H, I, J>
where
    App: Application,
{
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
    pub click_handler: DefaultClickHandler<App, Value, Item, B, A>,
}

impl<App, Value, Item, A, B, C, D, E, F, G, H, I, J> Element<App> for DropDown<App, Value, Item, A, B, C, D, E, F, G, H, I, J>
where
    App: Application,
    Value: PartialEq + 'static,
    Item: DropDownItem<Value> + 'static,
    // TODO: Is it nicer to take a selector here?
    A: Path<App, Vec<Item>>,
    // TODO: Is it nicer to take a selector here?
    B: Path<App, Value>,
    C: Selector<App, App::Color>,
    D: Selector<App, App::Color>,
    E: Selector<App, App::Color>,
    F: Selector<App, App::Color>,
    G: Selector<App, f32>,
    H: Selector<App, App::CornerRadius>,
    I: Selector<App, App::FontSize>,
    J: Selector<App, HorizontalAlignment>,
{
    fn create_layout_info(
        &mut self,
        state: &Context<App>,
        _: &mut ElementStore,
        _: &mut ElementIdGenerator,
        resolver: &mut Resolver,
    ) -> Self::LayoutInfo {
        let height = state.get(&self.height);
        let area = resolver.with_height(*height);

        let list_maximum_height = *state.get(&theme().drop_down().list_maximum_height());
        let border = *state.get(&theme().drop_down().list_border());

        self.click_handler.set_position_size(
            App::Position::new(area.left - border, area.top - border),
            App::Size::new(area.width + border * 2.0, list_maximum_height + border * 2.0),
        );

        Self::LayoutInfo { area }
    }

    fn layout_element<'a>(
        &'a self,
        state: &'a Context<App>,
        _: &'a ElementStore,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut Layout<'a, App>,
    ) {
        let is_hoverered = layout.is_area_hovered_and_active(layout_info.area);

        if is_hoverered {
            layout.add_click_area(layout_info.area, MouseButton::Left, &self.click_handler.overlay_element);
            layout.mark_hovered();
        }

        let background_color = match is_hoverered {
            true => *state.get(&self.hovered_background_color),
            false => *state.get(&self.background_color),
        };

        layout.add_rectangle(layout_info.area, *state.get(&self.corner_radius), background_color);

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
            *state.get(&self.font_size),
            foreground_color,
            *state.get(&self.text_alignment),
            *state.get(&theme().drop_down().button_vertical_alignment()),
        );
    }
}
