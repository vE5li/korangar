mod resolver;

use std::cell::RefCell;

use alignment::{HorizontalAlignment, VerticalAlignment};
use area::Area;
use num::Signed;
use rust_state::Context;

pub use self::resolver::{HeightBound, Resolver};
use crate::MouseMode;
use crate::application::{Appli, ClipTrait, PositionTrait, RenderLayer, SizeTrait};
use crate::element::id::ElementId;
use crate::event::{ClickAction, Event, EventQueue};

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

struct RectangleInsturction<App: Appli> {
    clip_layer: ClipLayerId,
    area: Area,
    corner_radius: App::CornerRadius,
    color: App::Color,
}

struct CheckboxInstruction<App: Appli> {
    clip_layer: ClipLayerId,
    area: Area,
    color: App::Color,
    state: bool,
}

struct TextInstruction<'a, App: Appli> {
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

struct LayoutLayer<'a, App: Appli> {
    rectangles: Vec<RectangleInsturction<App>>,
    texts: Vec<TextInstruction<'a, App>>,
    checkboxes: Vec<CheckboxInstruction<App>>,
    click_areas: Vec<ClickArea<'a, App>>,
    window_move_areas: Vec<WindowArea>,
    window_resize_areas: Vec<WindowArea>,
    window_close_areas: Vec<WindowArea>,
    scroll_areas: Vec<ScrollArea<'a>>,
    toggles: Vec<ToggleInstruction<'a>>,
    focus_areas: Vec<FocusArea>,
    input_handlers: Vec<&'a dyn InputHandler<App>>,
}

impl<App: Appli> Default for LayoutLayer<'_, App> {
    fn default() -> Self {
        Self {
            rectangles: Default::default(),
            texts: Default::default(),
            checkboxes: Default::default(),
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
pub struct ClipLayerId(usize);

pub struct ClipLayer<App: Appli> {
    parent: Option<ClipLayerId>,
    clip: App::Clip,
}

impl<App: Appli> Clone for ClipLayer<App> {
    fn clone(&self) -> Self {
        Self {
            parent: self.parent.clone(),
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

pub struct Layout<'a, App: Appli> {
    layers: Vec<LayoutLayer<'a, App>>,
    current_layer: usize,
    can_hover: bool,

    clip_layers: Vec<ClipLayer<App>>,
    active_clip_layers: Vec<ClipLayerId>,

    mouse_position: App::Position,
    focused_element: Option<ElementId>,
}

impl<'a, App: Appli> Layout<'a, App> {
    pub fn new(mouse_position: App::Position, focused_element: Option<ElementId>, can_hover: bool) -> Self {
        Self {
            layers: vec![LayoutLayer::default()],
            current_layer: 0,
            can_hover,

            clip_layers: vec![ClipLayer {
                parent: None,
                clip: App::Clip::unbound(),
            }],
            active_clip_layers: vec![ClipLayerId(0)],

            mouse_position,
            focused_element,
        }
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

    pub fn push_layer(&mut self) {
        self.current_layer += 1;

        if self.current_layer >= self.layers.len() {
            self.layers.push(LayoutLayer::default());
        }
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

    pub fn pop_layer(&mut self) {
        self.current_layer -= 1;
    }

    pub fn add_click_area(&mut self, area: Area, action: &'a dyn ClickAction<App>) {
        let clip_layer = self.active_clip_layers.last().copied().unwrap();

        self.layers[self.current_layer]
            .click_areas
            .push(ClickArea { clip_layer, area, action });
    }

    pub fn add_window_move_area(&mut self, area: Area, window_id: u64) {
        let clip_layer = self.active_clip_layers.last().copied().unwrap();

        self.layers[self.current_layer].window_move_areas.push(WindowArea {
            clip_layer,
            area,
            window_id,
        });
    }

    pub fn add_window_resize_area(&mut self, area: Area, window_id: u64) {
        let clip_layer = self.active_clip_layers.last().copied().unwrap();

        self.layers[self.current_layer].window_resize_areas.push(WindowArea {
            clip_layer,
            area,
            window_id,
        });
    }

    pub fn add_window_close_area(&mut self, area: Area, window_id: u64) {
        let clip_layer = self.active_clip_layers.last().copied().unwrap();

        self.layers[self.current_layer].window_close_areas.push(WindowArea {
            clip_layer,
            area,
            window_id,
        });
    }

    pub fn add_scroll_area(&mut self, area: Area, max_scroll: f32, cell: &'a RefCell<f32>) {
        let clip_layer = self.active_clip_layers.last().copied().unwrap();

        self.layers[self.current_layer].scroll_areas.push(ScrollArea {
            clip_layer,
            area,
            max_scroll,
            cell,
        });
    }

    pub fn add_toggle(&mut self, area: Area, cell: &'a RefCell<bool>) {
        let clip_layer = self.active_clip_layers.last().copied().unwrap();

        self.layers[self.current_layer]
            .toggles
            .push(ToggleInstruction { clip_layer, area, cell });
    }

    pub fn add_focus_area(&mut self, area: Area, element_id: ElementId) {
        let clip_layer = self.active_clip_layers.last().copied().unwrap();

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
        let clip_layer = self.active_clip_layers.last().copied().unwrap();

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
        let clip_layer = self.active_clip_layers.last().copied().unwrap();

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
        let clip_layer = self.active_clip_layers.last().copied().unwrap();

        self.layers[self.current_layer].checkboxes.push(CheckboxInstruction {
            clip_layer,
            area,
            color,
            state,
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
    ) -> bool {
        let mut clicked = false;

        for layer in self.layers.iter().rev() {
            for click_area in &layer.click_areas {
                // TODO: Check clip layer as well

                if click_position.left() >= click_area.area.x
                    && click_position.left() <= click_area.area.x + click_area.area.width
                    && click_position.top() >= click_area.area.y
                    && click_position.top() <= click_area.area.y + click_area.area.height
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
