mod resolver;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use alignment::{HorizontalAlignment, VerticalAlignment};
use area::Area;
use num::Signed;
use rust_state::Context;
use tooltip::{Tooltip, TooltipId};

pub use self::resolver::{HeightBound, Resolver};
use crate::MouseMode;
use crate::application::{Application, ClipTrait, PositionTrait, RenderLayer, SizeTrait};
use crate::element::id::ElementId;
use crate::event::{ClickAction, Event, EventQueue};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
}

pub mod alignment {
    #[derive(Clone, Copy)]
    pub enum HorizontalAlignment {
        Left { offset: f32 },
        Center { offset: f32 },
        Right { offset: f32 },
    }

    #[derive(Clone, Copy)]
    pub enum VerticalAlignment {
        Top { offset: f32 },
        Center { offset: f32 },
        Bottom { offset: f32 },
    }
}

pub mod area {
    // TODO: left + top
    #[derive(Debug, Clone, Copy)]
    pub struct Area {
        pub x: f32,
        pub y: f32,
        pub width: f32,
        pub height: f32,
    }

    // TODO: left + top
    #[derive(Debug, Clone, Copy)]
    pub struct PartialArea {
        pub x: f32,
        pub y: f32,
        pub width: f32,
        pub height: Option<f32>,
    }

    impl From<Area> for PartialArea {
        fn from(value: Area) -> Self {
            Self {
                x: value.x,
                y: value.y,
                width: value.width,
                height: Some(value.height),
            }
        }
    }
}

struct RectangleInsturction<App: Application> {
    clip_layer: ClipLayerId,
    area: Area,
    corner_radius: App::CornerRadius,
    color: App::Color,
}

struct CheckboxInstruction<App: Application> {
    clip_layer: ClipLayerId,
    area: Area,
    color: App::Color,
    state: bool,
}

struct TextInstruction<'a, App: Application> {
    clip_layer: ClipLayerId,
    area: Area,
    text: &'a str,
    font_size: App::FontSize,
    color: App::Color,
    horizontal_alignment: HorizontalAlignment,
    vertical_alignment: VerticalAlignment,
}

struct ClickArea<'a, App> {
    clip_layer: ClipLayerId,
    area: Area,
    mouse_button: MouseButton,
    action: &'a dyn ClickAction<App>,
}

struct WindowArea {
    clip_layer: ClipLayerId,
    area: Area,
    window_id: u64,
}

struct ScrollArea<'a> {
    clip_layer: ClipLayerId,
    area: Area,
    max_scroll: f32,
    cell: &'a RefCell<f32>,
}

struct ToggleInstruction<'a> {
    clip_layer: ClipLayerId,
    area: Area,
    cell: &'a RefCell<bool>,
}

struct FocusArea {
    clip_layer: ClipLayerId,
    area: Area,
    element_id: ElementId,
}

/// Handler for receiving keyboard input.
pub trait InputHandler<App> {
    fn handle_character(&self, state: &Context<App>, character: char);
}

struct LayoutLayer<'a, App: Application> {
    rectangles: Vec<RectangleInsturction<App>>,
    texts: Vec<TextInstruction<'a, App>>,
    checkboxes: Vec<CheckboxInstruction<App>>,
    custom_instructions: Vec<<App::Renderer as RenderLayer<App>>::CustomInstruction<'a>>,
    click_areas: Vec<ClickArea<'a, App>>,
    window_move_areas: Vec<WindowArea>,
    window_resize_areas: Vec<WindowArea>,
    window_close_areas: Vec<WindowArea>,
    scroll_areas: Vec<ScrollArea<'a>>,
    toggles: Vec<ToggleInstruction<'a>>,
    focus_areas: Vec<FocusArea>,
    input_handlers: Vec<&'a dyn InputHandler<App>>,
}

impl<App: Application> LayoutLayer<'_, App> {
    fn clear(&mut self) {
        self.rectangles.clear();
        self.texts.clear();
        self.checkboxes.clear();
        self.custom_instructions.clear();
        self.click_areas.clear();
        self.window_move_areas.clear();
        self.window_resize_areas.clear();
        self.window_close_areas.clear();
        self.scroll_areas.clear();
        self.toggles.clear();
        self.focus_areas.clear();
        self.input_handlers.clear();
    }
}

impl<App: Application> Default for LayoutLayer<'_, App> {
    fn default() -> Self {
        Self {
            rectangles: Default::default(),
            texts: Default::default(),
            checkboxes: Default::default(),
            custom_instructions: Default::default(),
            click_areas: Default::default(),
            window_move_areas: Default::default(),
            window_resize_areas: Default::default(),
            window_close_areas: Default::default(),
            scroll_areas: Default::default(),
            toggles: Default::default(),
            focus_areas: Default::default(),
            input_handlers: Default::default(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
// TODO: Make inner field private (maybe)
pub struct ClipLayerId(pub usize);

pub struct ClipLayer<App: Application> {
    parent: Option<ClipLayerId>,
    clip: App::Clip,
}

impl<App: Application> ClipLayer<App> {
    pub fn get(&self) -> App::Clip {
        self.clip
    }
}

impl<App: Application> Clone for ClipLayer<App> {
    fn clone(&self) -> Self {
        Self {
            parent: self.parent,
            clip: self.clip,
        }
    }
}

// TODO: If the clip layer handling remains internal to this type, the clip
// layer handle can be completely removed.
pub struct ClipLayerHandle(ClipLayerId);

impl Drop for ClipLayerHandle {
    fn drop(&mut self) {
        panic!("Clip layer was not applied");
    }
}

pub mod tooltip {
    use std::any::{Any, TypeId};

    use rust_state::RustState;

    use crate::application::Application;

    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub struct TooltipId(TypeId);

    pub trait TooltipExt {
        fn tooltip_id(&self) -> TooltipId;
    }

    impl<T> TooltipExt for T
    where
        T: Any,
    {
        fn tooltip_id(&self) -> TooltipId {
            TooltipId(T::type_id(&self))
        }
    }

    #[derive(RustState)]
    pub struct TooltipTheme<App>
    where
        App: Application + 'static,
    {
        pub background_color: App::Color,
        pub foreground_color: App::Color,
        pub font_size: App::FontSize,
        pub corner_radius: App::CornerRadius,
        pub border: f32,
        pub gap: f32,
        pub mouse_offset: f32,
    }

    pub struct Tooltip<'a> {
        pub text: &'a str,
        pub id: TooltipId,
    }
}

pub struct Layout<'a, App: Application> {
    layers: Vec<LayoutLayer<'a, App>>,
    current_layer: usize,
    can_hover: bool,

    clip_layers: Vec<ClipLayer<App>>,
    active_clip_layers: Vec<ClipLayerId>,

    mouse_position: App::Position,
    focused_element: Option<ElementId>,

    use_secondary_color: bool,

    tooltips: Vec<Tooltip<'a>>,
    tooltip_timers: BTreeMap<TooltipId, Instant>,
}

impl<App: Application> Default for Layout<'_, App> {
    fn default() -> Self {
        Self {
            layers: vec![LayoutLayer::default()],
            current_layer: 0,
            can_hover: false,

            clip_layers: vec![ClipLayer {
                parent: None,
                clip: App::Clip::unbound(),
            }],
            active_clip_layers: vec![ClipLayerId(0)],

            mouse_position: App::Position::new(0.0, 0.0),
            focused_element: None,

            use_secondary_color: false,

            tooltips: Vec::new(),
            tooltip_timers: BTreeMap::new(),
        }
    }
}

impl<'a, App: Application> Layout<'a, App> {
    /// This function is responsible for clearing any state that are captured by
    /// the 'a lifetime. Ideally, this function does not deallocate any
    /// memory since it will very likely need to be re-allocated on the next
    /// frame.
    ///
    /// This function is the reason that the `transmute` in the `Interface` is
    /// safe, so it should be implemented with care.
    pub fn clear(&mut self) {
        self.layers.iter_mut().for_each(LayoutLayer::clear);

        self.clip_layers.clear();
        self.clip_layers.push(ClipLayer {
            parent: None,
            clip: App::Clip::unbound(),
        });

        self.tooltips.clear();
    }

    pub fn update(&mut self, mouse_position: App::Position, focused_element: Option<ElementId>, can_hover: bool) {
        self.focused_element = focused_element;
        self.mouse_position = mouse_position;
        self.can_hover = can_hover;
    }

    pub fn is_hovered(&self) -> bool {
        !self.can_hover
    }

    pub fn mark_hovered(&mut self) {
        self.can_hover = false;
    }

    // TODO: Just expose `can_hover` as `mouse_free` or something hand have the
    // caller combine these themselves.
    pub fn is_area_hovered_and_active(&self, area: Area) -> bool {
        self.can_hover
            && self.mouse_position.left() >= area.x
            && self.mouse_position.top() >= area.y
            && self.mouse_position.left() <= area.x + area.width
            && self.mouse_position.top() <= area.y + area.height
    }

    pub fn is_area_hovered(&self, area: Area) -> bool {
        self.mouse_position.left() >= area.x
            && self.mouse_position.top() >= area.y
            && self.mouse_position.left() <= area.x + area.width
            && self.mouse_position.top() <= area.y + area.height
    }

    pub fn is_element_focused(&self, element_id: ElementId) -> bool {
        self.focused_element.is_some_and(|id| id == element_id)
    }

    fn push_layer(&mut self) {
        self.current_layer += 1;

        if self.current_layer >= self.layers.len() {
            self.layers.push(LayoutLayer::default());
        }
    }

    fn pop_layer(&mut self) {
        self.current_layer -= 1;
    }

    pub fn with_layer(&mut self, mut f: impl FnMut(&mut Self)) {
        self.push_layer();

        f(self);

        self.pop_layer();
    }

    fn new_clip_layer(&mut self) -> ClipLayerHandle {
        let id = ClipLayerId(self.clip_layers.len());
        let parent = *self.active_clip_layers.last().unwrap();

        self.clip_layers.push(ClipLayer {
            parent: Some(parent),
            clip: App::Clip::new(0.0, 0.0, 0.0, 0.0),
        });
        self.active_clip_layers.push(id);

        ClipLayerHandle(id)
    }

    fn set_layer_clip(&mut self, handle: ClipLayerHandle, area: Area) {
        let clip = App::Clip::new(area.x, area.y, area.x + area.width, area.y + area.height);

        self.clip_layers[handle.0.0].clip = clip;

        self.active_clip_layers.pop();

        std::mem::forget(handle);
    }

    pub fn with_clip_layer(&mut self, area: Area, mut f: impl FnMut(&mut Self)) {
        let handle = self.new_clip_layer();

        f(self);

        self.set_layer_clip(handle, area);
    }

    pub fn get_active_clip_layer(&self) -> ClipLayerId {
        self.active_clip_layers.last().copied().unwrap()
    }

    pub fn with_secondary_background(&mut self, f: impl Fn(&mut Self)) -> bool {
        let previous = self.use_secondary_color;
        self.use_secondary_color = !self.use_secondary_color;

        f(self);

        self.use_secondary_color = previous;
        previous
    }

    pub fn add_click_area(&mut self, area: Area, button: MouseButton, action: &'a dyn ClickAction<App>) {
        let clip_layer = self.get_active_clip_layer();

        self.layers[self.current_layer].click_areas.push(ClickArea {
            clip_layer,
            area,
            mouse_button: button,
            action,
        });
    }

    pub fn add_window_move_area(&mut self, area: Area, window_id: u64) {
        let clip_layer = self.get_active_clip_layer();

        self.layers[self.current_layer].window_move_areas.push(WindowArea {
            clip_layer,
            area,
            window_id,
        });
    }

    pub fn add_window_resize_area(&mut self, area: Area, window_id: u64) {
        let clip_layer = self.get_active_clip_layer();

        self.layers[self.current_layer].window_resize_areas.push(WindowArea {
            clip_layer,
            area,
            window_id,
        });
    }

    pub fn add_window_close_area(&mut self, area: Area, window_id: u64) {
        let clip_layer = self.get_active_clip_layer();

        self.layers[self.current_layer].window_close_areas.push(WindowArea {
            clip_layer,
            area,
            window_id,
        });
    }

    pub fn add_scroll_area(&mut self, area: Area, max_scroll: f32, cell: &'a RefCell<f32>) {
        let clip_layer = self.get_active_clip_layer();

        self.layers[self.current_layer].scroll_areas.push(ScrollArea {
            clip_layer,
            area,
            max_scroll,
            cell,
        });
    }

    pub fn add_toggle(&mut self, area: Area, cell: &'a RefCell<bool>) {
        let clip_layer = self.get_active_clip_layer();

        self.layers[self.current_layer]
            .toggles
            .push(ToggleInstruction { clip_layer, area, cell });
    }

    pub fn add_focus_area(&mut self, area: Area, element_id: ElementId) {
        let clip_layer = self.get_active_clip_layer();

        self.layers[self.current_layer].focus_areas.push(FocusArea {
            clip_layer,
            area,
            element_id,
        });
    }

    pub fn add_input_handler(&mut self, input_handler: &'a dyn InputHandler<App>) {
        self.layers[self.current_layer].input_handlers.push(input_handler);
    }

    pub fn add_rectangle(&mut self, area: Area, corner_radius: App::CornerRadius, color: App::Color) {
        let clip_layer = self.get_active_clip_layer();

        self.layers[self.current_layer].rectangles.push(RectangleInsturction {
            clip_layer,
            area,
            corner_radius,
            color,
        });
    }

    pub fn add_text(
        &mut self,
        area: Area,
        text: &'a str,
        font_size: App::FontSize,
        color: App::Color,
        horizontal_alignment: HorizontalAlignment,
        vertical_alignment: VerticalAlignment,
    ) {
        let clip_layer = self.get_active_clip_layer();

        self.layers[self.current_layer].texts.push(TextInstruction {
            clip_layer,
            area,
            text,
            font_size,
            color,
            horizontal_alignment,
            vertical_alignment,
        });
    }

    pub fn add_checkbox(&mut self, area: Area, color: App::Color, state: bool) {
        let clip_layer = self.get_active_clip_layer();

        self.layers[self.current_layer].checkboxes.push(CheckboxInstruction {
            clip_layer,
            area,
            color,
            state,
        });
    }

    pub fn add_custom_instruction(&mut self, instruction: <App::Renderer as RenderLayer<App>>::CustomInstruction<'a>) {
        self.layers[self.current_layer].custom_instructions.push(instruction);
    }

    pub fn add_tooltip(&mut self, text: &'a str, id: TooltipId) {
        let tooltip = Tooltip { text, id };
        self.tooltips.push(tooltip);

        // If the tooltip was not present last frame start the timer now.
        self.tooltip_timers.entry(id).or_insert_with(Instant::now);
    }

    /// Update tooltips and collect those that have been registered for some
    /// time. Those are the tooltips that will be rendered to the screen.
    pub fn update_tooltips(&mut self, tooltips: &mut Vec<&'a str>) {
        self.tooltip_timers.retain(|id, timer| {
            if let Some(tooltip) = self.tooltips.iter().find(|tooltip| tooltip.id == *id) {
                if timer.elapsed() > Duration::from_secs(1) {
                    tooltips.push(tooltip.text);
                }

                true
            } else {
                // Remove any timers for tooltips that are no longer registered.
                false
            }
        });
    }

    pub fn render(&mut self, renderer: &App::Renderer) {
        fn combine_clip<T: ClipTrait>(clip: T, other: T) -> T {
            T::new(
                clip.left().max(other.left()),
                clip.top().max(other.top()),
                clip.right().min(other.right()),
                clip.bottom().min(other.bottom()),
            )
        }

        for index in 0..self.clip_layers.len() {
            let layer = self.clip_layers[index].clone();
            let layer_id = ClipLayerId(index);

            for child_layer in &mut self.clip_layers[index + 1..] {
                if child_layer.parent.is_some_and(|id| id == layer_id) {
                    child_layer.clip = combine_clip(child_layer.clip, layer.clip);
                }
            }
        }

        for layer in self.layers.iter_mut() {
            layer.rectangles.drain(..).for_each(
                |RectangleInsturction {
                     clip_layer,
                     area,
                     corner_radius,
                     color,
                 }: RectangleInsturction<App>| {
                    let clip = self.clip_layers[clip_layer.0].clip;

                    renderer.render_rectangle(
                        App::Position::new(area.x, area.y),
                        App::Size::new(area.width, area.height),
                        clip,
                        corner_radius,
                        color,
                    );
                },
            );

            layer.checkboxes.drain(..).for_each(
                |CheckboxInstruction {
                     clip_layer,
                     area,
                     color,
                     state,
                 }: CheckboxInstruction<App>| {
                    let clip = self.clip_layers[clip_layer.0].clip;

                    renderer.render_checkbox(
                        App::Position::new(area.x, area.y),
                        App::Size::new(area.width, area.height),
                        clip,
                        color,
                        state,
                    );
                },
            );

            layer.custom_instructions.drain(..).for_each(|instruction| {
                renderer.render_custom(instruction, &self.clip_layers);
            });

            layer.texts.drain(..).for_each(
                |TextInstruction {
                     clip_layer,
                     area,
                     text,
                     font_size,
                     color,
                     horizontal_alignment,
                     vertical_alignment,
                 }: TextInstruction<'_, App>| {
                    let text_size = renderer.get_text_dimensions(text, font_size, area.width);
                    let clip = self.clip_layers[clip_layer.0].clip;

                    let top_offset = match vertical_alignment {
                        VerticalAlignment::Top { offset } => offset,
                        VerticalAlignment::Center { offset } => {
                            let top_offset = (area.height - text_size.height()) / 2.0;
                            top_offset + offset
                        }
                        VerticalAlignment::Bottom { offset } => {
                            let top_offset = area.height - text_size.height();
                            top_offset - offset
                        }
                    };

                    let left_offset = match horizontal_alignment {
                        HorizontalAlignment::Left { offset } => offset,
                        HorizontalAlignment::Center { offset } => {
                            let left_offset = (area.width - text_size.width()) / 2.0;
                            left_offset + offset
                        }
                        HorizontalAlignment::Right { offset } => {
                            let top_offset = area.width - text_size.width();
                            top_offset - offset
                        }
                    };

                    renderer.render_text(
                        text,
                        App::Position::new(area.x + left_offset, area.y + top_offset),
                        clip,
                        color,
                        font_size,
                    );
                },
            );
        }
    }

    pub fn do_click(
        &self,
        state: &Context<App>,
        queue: &mut EventQueue<App>,
        focused_element: &mut Option<ElementId>,
        mouse_mode: &mut MouseMode,
        click_position: App::Position,
        mouse_button: MouseButton,
    ) -> bool {
        let mut clicked = false;

        for layer in self.layers.iter().rev() {
            for click_area in &layer.click_areas {
                // TODO: Check clip layer as well

                if click_position.left() >= click_area.area.x
                    && click_position.left() <= click_area.area.x + click_area.area.width
                    && click_position.top() >= click_area.area.y
                    && click_position.top() <= click_area.area.y + click_area.area.height
                    && click_area.mouse_button == mouse_button
                {
                    click_area.action.execute(state, queue);
                    clicked = true;
                }
            }

            for toggle in &layer.toggles {
                if click_position.left() >= toggle.area.x
                    && click_position.left() <= toggle.area.x + toggle.area.width
                    && click_position.top() >= toggle.area.y
                    && click_position.top() <= toggle.area.y + toggle.area.height
                {
                    let mut reference = toggle.cell.borrow_mut();
                    *reference = !*reference;
                    clicked = true;
                }
            }

            for window_move_area in &layer.window_move_areas {
                // TODO: Check clip layer as well

                if click_position.left() >= window_move_area.area.x
                    && click_position.left() <= window_move_area.area.x + window_move_area.area.width
                    && click_position.top() >= window_move_area.area.y
                    && click_position.top() <= window_move_area.area.y + window_move_area.area.height
                {
                    *mouse_mode = MouseMode::MovingWindow {
                        window_id: window_move_area.window_id,
                    };
                    clicked = true;
                }
            }

            for window_resize_area in &layer.window_resize_areas {
                // TODO: Check clip layer as well

                if click_position.left() >= window_resize_area.area.x
                    && click_position.left() <= window_resize_area.area.x + window_resize_area.area.width
                    && click_position.top() >= window_resize_area.area.y
                    && click_position.top() <= window_resize_area.area.y + window_resize_area.area.height
                {
                    *mouse_mode = MouseMode::ResizingWindow {
                        window_id: window_resize_area.window_id,
                    };
                    clicked = true;
                }
            }

            for window_close_area in &layer.window_close_areas {
                // TODO: Check clip layer as well

                if click_position.left() >= window_close_area.area.x
                    && click_position.left() <= window_close_area.area.x + window_close_area.area.width
                    && click_position.top() >= window_close_area.area.y
                    && click_position.top() <= window_close_area.area.y + window_close_area.area.height
                {
                    queue.queue(Event::CloseWindow {
                        window_id: window_close_area.window_id,
                    });
                    clicked = true;
                }
            }

            for focus_area in &layer.focus_areas {
                // TODO: Check clip layer as well

                if click_position.left() >= focus_area.area.x
                    && click_position.left() <= focus_area.area.x + focus_area.area.width
                    && click_position.top() >= focus_area.area.y
                    && click_position.top() <= focus_area.area.y + focus_area.area.height
                {
                    *focused_element = Some(focus_area.element_id);
                    clicked = true;
                }
            }
        }

        clicked
    }

    pub fn do_scroll(&self, mouse_position: App::Position, delta: f32) -> bool {
        for layer in self.layers.iter().rev() {
            for scroll_area in &layer.scroll_areas {
                // TODO: Check clip layer as well

                if mouse_position.left() >= scroll_area.area.x
                    && mouse_position.left() <= scroll_area.area.x + scroll_area.area.width
                    && mouse_position.top() >= scroll_area.area.y
                    && mouse_position.top() <= scroll_area.area.y + scroll_area.area.height
                {
                    let mut current_scroll = scroll_area.cell.borrow_mut();

                    // Don't try to scroll stuff that is already at the min or max scroll value.
                    if delta.is_negative() && *current_scroll >= scroll_area.max_scroll || delta.is_positive() && *current_scroll <= 0.0 {
                        continue;
                    }

                    *current_scroll = (*current_scroll - delta).max(0.0).min(scroll_area.max_scroll);

                    return true;
                }
            }
        }

        false
    }

    pub fn input_character(&self, state: &Context<App>, character: char) {
        for layer in &self.layers {
            for input_handler in &layer.input_handlers {
                input_handler.handle_character(state, character);
            }
        }
    }
}
