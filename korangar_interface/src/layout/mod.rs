mod resolver;

pub mod alignment;
pub mod area;
pub mod tooltip;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use alignment::{HorizontalAlignment, VerticalAlignment};
use area::Area;
use num::Signed;
use rust_state::Context;
use tooltip::{Tooltip, TooltipId};

pub use self::resolver::{Resolver, ResolverSet};
use crate::MouseMode;
use crate::application::{Application, Clip, CornerDiameter, FontSize, Position, RenderLayer, Size, TextLayouter};
use crate::element::id::{ElementId, FocusId};
use crate::event::{ClickHandler, EventQueue};

// Rename this to ButtonPress or something.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    DoubleLeft,
    DoubleRight,
}

/// Different modes for resizing a window.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ResizeMode {
    /// Only update the horizontal dimensions of the window.
    Horizontal,
    /// Only update the vertical dimensions of the window.
    Vertical,
    /// Update both the horizontal and vertical dimensions of the window.
    Both,
}

/// Icons that can be rendered from the [`Layout`].
#[derive(Clone, Copy)]
pub enum Icon<App: Application> {
    /// Arrow used for collapsable components.
    ExpandArrow { expanded: bool },
    /// Checkbox used for toggleable components.
    Checkbox { checked: bool },
    /// Eye for hidable components.
    Eye { open: bool },
    /// Trash can for clear buttons.
    TrashCan,
    /// Application defined custom icons.
    Custom {
        icon: <App::Renderer as RenderLayer<App>>::CustomIcon,
    },
}

struct RectangleInstruction<App: Application> {
    clip_layer: ClipLayerId,
    area: Area,
    corner_diameter: App::CornerDiameter,
    color: App::Color,
}

struct IconInstruction<App: Application> {
    clip_layer: ClipLayerId,
    icon: Icon<App>,
    area: Area,
    color: App::Color,
}

struct TextInstruction<'a, App: Application> {
    clip_layer: ClipLayerId,
    area: Area,
    text: &'a str,
    font_size: App::FontSize,
    color: App::Color,
    horizontal_alignment: HorizontalAlignment,
    vertical_alignment: VerticalAlignment,
    overflow_behavior: App::OverflowBehavior,
}

struct ClickArea<'a, App> {
    clip_layer: ClipLayerId,
    area: Area,
    mouse_button: MouseButton,
    handler: &'a dyn ClickHandler<App>,
}

struct DropArea<'a, App> {
    clip_layer: ClipLayerId,
    area: Area,
    handler: &'a dyn DropHandler<App>,
}

struct ScrollArea<'a> {
    clip_layer: ClipLayerId,
    area: Area,
    max_scroll: f32,
    cell: &'a RefCell<f32>,
}

/// Handler for receiving keyboard input.
pub trait InputHandler<App: Application> {
    fn handle_character(&self, state: &Context<App>, queue: &mut EventQueue<App>, character: char);
}

/// Handler for handling dropping a resource.
pub trait DropHandler<App: Application> {
    fn handle_drop(&self, state: &Context<App>, queue: &mut EventQueue<App>, mouse_mode: &MouseMode<App>);
}

struct LayoutLayer<'a, App: Application> {
    rectangle_instructions: Vec<RectangleInstruction<App>>,
    text_instructions: Vec<TextInstruction<'a, App>>,
    icon_instructions: Vec<IconInstruction<App>>,
    custom_instructions: Vec<<App::Renderer as RenderLayer<App>>::CustomInstruction<'a>>,
    click_areas: Vec<ClickArea<'a, App>>,
    drop_areas: Vec<DropArea<'a, App>>,
    scroll_areas: Vec<ScrollArea<'a>>,
    input_handlers: Vec<&'a dyn InputHandler<App>>,
}

impl<App: Application> LayoutLayer<'_, App> {
    fn clear(&mut self) {
        self.rectangle_instructions.clear();
        self.text_instructions.clear();
        self.icon_instructions.clear();
        self.custom_instructions.clear();
        self.click_areas.clear();
        self.drop_areas.clear();
        self.scroll_areas.clear();
        self.input_handlers.clear();
    }
}

impl<App: Application> Default for LayoutLayer<'_, App> {
    fn default() -> Self {
        Self {
            rectangle_instructions: Default::default(),
            text_instructions: Default::default(),
            icon_instructions: Default::default(),
            custom_instructions: Default::default(),
            click_areas: Default::default(),
            drop_areas: Default::default(),
            scroll_areas: Default::default(),
            input_handlers: Default::default(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ClipLayerId(usize);

impl ClipLayerId {
    pub fn as_index(&self) -> usize {
        self.0
    }
}

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

pub struct HoverCheck {
    area: Area,
    check_mouse_mode: bool,
    mark_after: bool,
}

impl HoverCheck {
    pub(crate) fn new(area: Area) -> Self {
        Self {
            area,
            check_mouse_mode: true,
            mark_after: true,
        }
    }

    pub fn any_mouse_mode(&mut self) -> &mut Self {
        self.check_mouse_mode = false;
        self
    }

    pub fn dont_mark(&mut self) -> &mut Self {
        self.mark_after = false;
        self
    }

    /// This currently doesn't respect the clip since we don't have the clip
    /// when performing this check. Maybe this will be changed in the
    /// future.
    pub fn run<App: Application>(&mut self, layout: &mut WindowLayout<'_, App>) -> bool {
        let is_hovered = layout.can_be_hovered
            && (!self.check_mouse_mode || layout.mouse_mode.as_ref().unwrap().is_default())
            && layout.mouse_position.left() >= self.area.left
            && layout.mouse_position.top() >= self.area.top
            && layout.mouse_position.left() <= self.area.left + self.area.width
            && layout.mouse_position.top() <= self.area.top + self.area.height;

        layout.can_be_hovered &= !(is_hovered && self.mark_after);

        is_hovered
    }
}

/// Internal helper for bounding an area to a clip.
fn apply_clip<T: Clip>(clip: T, area: Area) -> T {
    T::new(
        area.left.max(clip.left()),
        area.top.max(clip.top()),
        (area.left + area.width).min(clip.right()),
        (area.top + area.height).min(clip.bottom()),
    )
}

/// Internal helper for combining two clip.
fn combine_clip<T: Clip>(clip: T, other: T) -> T {
    T::new(
        clip.left().max(other.left()),
        clip.top().max(other.top()),
        clip.right().min(other.right()),
        clip.bottom().min(other.bottom()),
    )
}

pub struct WindowLayout<'a, App: Application> {
    layers: Vec<LayoutLayer<'a, App>>,
    current_layer: usize,
    is_hovered: bool,
    can_be_hovered: bool,

    clip_layers: Vec<ClipLayer<App>>,
    active_clip_layers: Vec<ClipLayerId>,

    mouse_position: App::Position,
    focused_element: Option<ElementId>,

    use_secondary_color: bool,

    tooltips: Vec<Tooltip<'a>>,
    tooltip_timers: BTreeMap<TooltipId, Instant>,

    focus_id_lookup: BTreeMap<FocusId, ElementId>,

    window_position: App::Position,
    interface_scaling: f32,

    mouse_mode: Option<&'a MouseMode<App>>,
}

impl<App: Application> Default for WindowLayout<'_, App> {
    fn default() -> Self {
        Self {
            layers: vec![LayoutLayer::default()],
            current_layer: 0,
            is_hovered: false,
            can_be_hovered: false,

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

            focus_id_lookup: BTreeMap::new(),

            window_position: App::Position::new(0.0, 0.0),
            interface_scaling: 1.0,

            mouse_mode: None,
        }
    }
}

impl<'a, App: Application> WindowLayout<'a, App> {
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
        self.focus_id_lookup.clear();
        self.mouse_mode = None;
    }

    pub fn update(
        &mut self,
        interface_scaling: f32,
        window_position: App::Position,
        mouse_position: App::Position,
        focused_element: Option<ElementId>,
        can_be_hovered: bool,
        mouse_mode: &'a MouseMode<App>,
    ) {
        self.interface_scaling = interface_scaling;
        self.window_position = window_position;
        self.mouse_position = App::Position::new(
            (mouse_position.left() - self.window_position.left()) / interface_scaling + self.window_position.left(),
            (mouse_position.top() - self.window_position.top()) / interface_scaling + self.window_position.top(),
        );
        self.focused_element = focused_element;
        self.is_hovered = false;
        self.can_be_hovered = can_be_hovered;
        self.mouse_mode = Some(mouse_mode);
    }

    pub fn get_interface_scaling(&self) -> f32 {
        self.interface_scaling
    }

    pub fn scale_area(&self, area: Area) -> Area {
        Area {
            left: (area.left - self.window_position.left()) * self.interface_scaling + self.window_position.left(),
            top: (area.top - self.window_position.top()) * self.interface_scaling + self.window_position.top(),
            width: area.width * self.interface_scaling,
            height: area.height * self.interface_scaling,
        }
    }

    pub fn get_mouse_position(&self) -> App::Position {
        self.mouse_position
    }

    pub fn get_mouse_mode(&self) -> &MouseMode<App> {
        self.mouse_mode.as_ref().unwrap()
    }

    pub fn is_hovered(&self) -> bool {
        self.is_hovered
    }

    /// Mark the window as hovered. This is done automatically if the mouse mode
    /// is default but for other modes this has to be called to make areas
    /// work.
    pub fn set_hovered(&mut self) {
        self.is_hovered = true;
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

    fn new_clip_layer(&mut self) -> ClipLayerId {
        let id = ClipLayerId(self.clip_layers.len());
        let parent = *self.active_clip_layers.last().unwrap();

        self.clip_layers.push(ClipLayer {
            parent: Some(parent),
            clip: App::Clip::new(0.0, 0.0, 0.0, 0.0),
        });
        self.active_clip_layers.push(id);

        id
    }

    fn set_layer_clip(&mut self, clip_layer_id: ClipLayerId, area: Area) {
        let area = self.scale_area(area);
        let clip = App::Clip::new(area.left, area.top, area.left + area.width, area.top + area.height);

        self.clip_layers[clip_layer_id.0].clip = clip;

        self.active_clip_layers.pop();
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

    pub fn add_click_area(&mut self, area: Area, button: MouseButton, handler: &'a dyn ClickHandler<App>) {
        let clip_layer = self.get_active_clip_layer();
        let area = self.scale_area(area);

        self.layers[self.current_layer].click_areas.push(ClickArea {
            clip_layer,
            area,
            mouse_button: button,
            handler,
        });
    }

    pub fn add_drop_area(&mut self, area: Area, handler: &'a dyn DropHandler<App>) {
        let clip_layer = self.get_active_clip_layer();
        let area = self.scale_area(area);

        self.layers[self.current_layer]
            .drop_areas
            .push(DropArea { clip_layer, area, handler });
    }

    pub fn add_scroll_area(&mut self, area: Area, max_scroll: f32, cell: &'a RefCell<f32>) {
        let clip_layer = self.get_active_clip_layer();
        let area = self.scale_area(area);

        self.layers[self.current_layer].scroll_areas.push(ScrollArea {
            clip_layer,
            area,
            max_scroll,
            cell,
        });
    }

    pub fn add_input_handler(&mut self, input_handler: &'a dyn InputHandler<App>) {
        self.layers[self.current_layer].input_handlers.push(input_handler);
    }

    pub fn add_rectangle(&mut self, area: Area, corner_diameter: App::CornerDiameter, color: App::Color) {
        let clip_layer = self.get_active_clip_layer();
        let area = self.scale_area(area);
        let corner_diameter = corner_diameter.scaled(self.interface_scaling);

        self.layers[self.current_layer].rectangle_instructions.push(RectangleInstruction {
            clip_layer,
            area,
            corner_diameter,
            color,
        });
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_text(
        &mut self,
        area: Area,
        text: &'a str,
        font_size: App::FontSize,
        color: App::Color,
        horizontal_alignment: HorizontalAlignment,
        vertical_alignment: VerticalAlignment,
        overflow_behavior: App::OverflowBehavior,
    ) {
        let clip_layer = self.get_active_clip_layer();
        let area = self.scale_area(area);
        let font_size = font_size.scaled(self.interface_scaling);
        let horizontal_alignment = horizontal_alignment.scaled(self.interface_scaling);
        let vertical_alignment = vertical_alignment.scaled(self.interface_scaling);

        self.layers[self.current_layer].text_instructions.push(TextInstruction {
            clip_layer,
            area,
            text,
            font_size,
            color,
            horizontal_alignment,
            vertical_alignment,
            overflow_behavior,
        });
    }

    pub fn add_icon(&mut self, area: Area, icon: Icon<App>, color: App::Color) {
        let clip_layer = self.get_active_clip_layer();
        let area = self.scale_area(area);

        self.layers[self.current_layer].icon_instructions.push(IconInstruction {
            clip_layer,
            area,
            icon,
            color,
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

    pub fn register_focus_id(&mut self, focus_id: FocusId, element_id: ElementId) {
        self.focus_id_lookup.insert(focus_id, element_id);
    }

    pub fn try_resolve_focus_id(&self, focus_id: FocusId) -> Option<ElementId> {
        self.focus_id_lookup.get(&focus_id).copied()
    }

    /// Update tooltips and collect those that have been registered for some
    /// time. Those are the tooltips that will be rendered to the screen.
    pub fn update_tooltips(&mut self, tooltips: &mut Vec<&'a str>) {
        self.tooltip_timers.retain(|id, timer| {
            let mut found = false;

            self.tooltips.iter().filter(|tooltip| tooltip.id == *id).for_each(|tooltip| {
                if timer.elapsed() > Duration::from_secs(1) {
                    tooltips.push(tooltip.text);
                }

                found = true;
            });

            found
        });
    }

    #[cfg(feature = "debug")]
    pub fn render_click_areas(&self, renderer: &App::Renderer, color: App::Color) {
        self.layers.iter().for_each(|layer| {
            layer.click_areas.iter().for_each(|click_area| {
                renderer.render_rectangle(
                    App::Position::new(click_area.area.left, click_area.area.top),
                    App::Size::new(click_area.area.width, click_area.area.height),
                    App::Clip::unbound(),
                    App::CornerDiameter::new(0.0, 0.0, 0.0, 0.0),
                    color,
                );
            });
        });
    }

    #[cfg(feature = "debug")]
    pub fn render_drop_areas(&self, renderer: &App::Renderer, color: App::Color) {
        self.layers.iter().for_each(|layer| {
            layer.drop_areas.iter().for_each(|drop_area| {
                renderer.render_rectangle(
                    App::Position::new(drop_area.area.left, drop_area.area.top),
                    App::Size::new(drop_area.area.width, drop_area.area.height),
                    App::Clip::unbound(),
                    App::CornerDiameter::new(0.0, 0.0, 0.0, 0.0),
                    color,
                );
            });
        });
    }

    #[cfg(feature = "debug")]
    pub fn render_scroll_areas(&self, renderer: &App::Renderer, color: App::Color) {
        self.layers.iter().for_each(|layer| {
            layer.scroll_areas.iter().for_each(|scroll_area| {
                renderer.render_rectangle(
                    App::Position::new(scroll_area.area.left, scroll_area.area.top),
                    App::Size::new(scroll_area.area.width, scroll_area.area.height),
                    App::Clip::unbound(),
                    App::CornerDiameter::new(0.0, 0.0, 0.0, 0.0),
                    color,
                );
            });
        });
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("render layout"))]
    pub fn render(&mut self, renderer: &App::Renderer, text_layouter: &App::TextLayouter) {
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
            #[cfg(feature = "debug")]
            korangar_debug::profile_block!("render layer");

            layer.rectangle_instructions.drain(..).for_each(
                |RectangleInstruction {
                     clip_layer,
                     area,
                     corner_diameter,
                     color,
                 }: RectangleInstruction<App>| {
                    #[cfg(feature = "debug")]
                    korangar_debug::profile_block!("rectangle instruction");

                    let clip = self.clip_layers[clip_layer.0].clip;

                    renderer.render_rectangle(
                        App::Position::new(area.left, area.top),
                        App::Size::new(area.width, area.height),
                        clip,
                        corner_diameter,
                        color,
                    );
                },
            );

            layer.icon_instructions.drain(..).for_each(
                |IconInstruction {
                     clip_layer,
                     icon,
                     area,
                     color,
                 }: IconInstruction<App>| {
                    #[cfg(feature = "debug")]
                    korangar_debug::profile_block!("icon instruction");

                    let clip = self.clip_layers[clip_layer.0].clip;

                    renderer.render_icon(
                        App::Position::new(area.left, area.top),
                        App::Size::new(area.width, area.height),
                        clip,
                        icon,
                        color,
                    );
                },
            );

            layer.custom_instructions.drain(..).for_each(|instruction| {
                #[cfg(feature = "debug")]
                korangar_debug::profile_block!("custom instruction");

                renderer.render_custom(instruction, &self.clip_layers);
            });

            layer.text_instructions.drain(..).for_each(
                |TextInstruction {
                     clip_layer,
                     area,
                     text,
                     font_size,
                     color,
                     horizontal_alignment,
                     vertical_alignment,
                     overflow_behavior,
                 }: TextInstruction<'_, App>| {
                    #[cfg(feature = "debug")]
                    korangar_debug::profile_block!("text instruction");

                    let clip = self.clip_layers[clip_layer.0].clip;

                    let available_width = match horizontal_alignment {
                        HorizontalAlignment::Left { offset, border } => area.width - offset - border,
                        HorizontalAlignment::Center { border, .. } => area.width - border * 2.0,
                        HorizontalAlignment::Right { offset, border } => area.width - offset - border,
                    };

                    let (text_size, font_size) = text_layouter.get_text_dimensions(text, font_size, available_width, overflow_behavior);

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
                        HorizontalAlignment::Left { offset, .. } => offset,
                        HorizontalAlignment::Center { offset, .. } => {
                            let left_offset = (area.width - text_size.width()) / 2.0;
                            left_offset + offset
                        }
                        HorizontalAlignment::Right { offset, .. } => {
                            let top_offset = area.width - text_size.width();
                            top_offset - offset
                        }
                    };

                    renderer.render_text(
                        text,
                        App::Position::new(area.left + left_offset, area.top + top_offset),
                        available_width,
                        clip,
                        color,
                        font_size,
                    );
                },
            );
        }
    }

    pub fn handle_click(
        &self,
        state: &Context<App>,
        queue: &mut EventQueue<App>,
        click_position: App::Position,
        mouse_button: MouseButton,
    ) -> bool {
        let mut clicked = false;

        for layer in self.layers.iter().rev() {
            for click_area in &layer.click_areas {
                let clip = self.clip_layers[click_area.clip_layer.0].clip;
                let clip = apply_clip(clip, click_area.area);

                if click_area.mouse_button == mouse_button
                    && click_position.left() >= clip.left()
                    && click_position.left() <= clip.right()
                    && click_position.top() >= clip.top()
                    && click_position.top() <= clip.bottom()
                {
                    click_area.handler.execute(state, queue);
                    clicked = true;
                }
            }
        }

        clicked
    }

    pub fn handle_drop(
        &self,
        state: &Context<App>,
        queue: &mut EventQueue<App>,
        drop_position: App::Position,
        mouse_mode: &'a MouseMode<App>,
    ) -> bool {
        let mut handled = false;

        for layer in self.layers.iter().rev() {
            for drop_area in &layer.drop_areas {
                let clip = self.clip_layers[drop_area.clip_layer.0].clip;
                let clip = apply_clip(clip, drop_area.area);

                if drop_position.left() >= clip.left()
                    && drop_position.left() <= clip.right()
                    && drop_position.top() >= clip.top()
                    && drop_position.top() <= clip.bottom()
                {
                    drop_area.handler.handle_drop(state, queue, mouse_mode);
                    handled = true;
                }
            }
        }

        handled
    }

    pub fn handle_scroll(&self, mouse_position: App::Position, delta: f32) -> bool {
        for layer in self.layers.iter().rev() {
            for scroll_area in &layer.scroll_areas {
                let clip = self.clip_layers[scroll_area.clip_layer.0].clip;
                let clip = apply_clip(clip, scroll_area.area);

                if mouse_position.left() >= clip.left()
                    && mouse_position.left() <= clip.right()
                    && mouse_position.top() >= clip.top()
                    && mouse_position.top() <= clip.bottom()
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

    pub fn handle_character(&self, state: &Context<App>, queue: &mut EventQueue<App>, character: char) -> bool {
        let mut input_handled = false;

        for layer in &self.layers {
            for input_handler in &layer.input_handlers {
                input_handler.handle_character(state, queue, character);
                input_handled = true;
            }
        }

        input_handled
    }
}
