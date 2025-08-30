mod resolver;

pub mod alignment;
pub mod area;
pub mod tooltip;

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use alignment::{HorizontalAlignment, VerticalAlignment};
use area::Area;
use rust_state::Context;
use tooltip::{Tooltip, TooltipId};

pub use self::resolver::{Resolver, ResolverSet};
use crate::MouseMode;
use crate::application::{Application, Clip, CornerDiameter, FontSize, Position, RenderLayer, ShadowPadding, Size, TextLayouter};
use crate::element::id::{ElementId, FocusId};
use crate::event::{ClickHandler, DropHandler, EventQueue, InputHandler, ScrollHandler};

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
    clip_id: ClipId,
    area: Area,
    corner_diameter: App::CornerDiameter,
    color: App::Color,
    shadow_color: App::Color,
    shadow_padding: App::ShadowPadding,
}

struct IconInstruction<App: Application> {
    clip_id: ClipId,
    icon: Icon<App>,
    area: Area,
    color: App::Color,
}

struct TextInstruction<'a, App: Application> {
    clip_id: ClipId,
    area: Area,
    text: &'a str,
    font_size: App::FontSize,
    color: App::Color,
    highlight_color: App::Color,
    horizontal_alignment: HorizontalAlignment,
    vertical_alignment: VerticalAlignment,
    overflow_behavior: App::OverflowBehavior,
}

struct LayoutLayer<'a, App: Application> {
    rectangle_instructions: Vec<RectangleInstruction<App>>,
    text_instructions: Vec<TextInstruction<'a, App>>,
    icon_instructions: Vec<IconInstruction<App>>,
    custom_instructions: Vec<<App::Renderer as RenderLayer<App>>::CustomInstruction<'a>>,
    click_handlers: Vec<(MouseButton, &'a dyn ClickHandler<App>)>,
    drop_handlers: Vec<&'a dyn DropHandler<App>>,
    scroll_handlers: Vec<&'a dyn ScrollHandler<App>>,
    input_handlers: Vec<&'a dyn InputHandler<App>>,
}

impl<App: Application> LayoutLayer<'_, App> {
    fn clear(&mut self) {
        self.rectangle_instructions.clear();
        self.text_instructions.clear();
        self.icon_instructions.clear();
        self.custom_instructions.clear();
        self.click_handlers.clear();
        self.drop_handlers.clear();
        self.scroll_handlers.clear();
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
            click_handlers: Default::default(),
            drop_handlers: Default::default(),
            scroll_handlers: Default::default(),
            input_handlers: Default::default(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ClipId(usize);

impl ClipId {
    pub fn as_index(&self) -> usize {
        self.0
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

    pub fn run<App: Application>(&mut self, layout: &mut WindowLayout<'_, App>) -> bool {
        let clip = &layout.clips[layout.active_clips.last().unwrap().0];

        let is_hovered = layout.can_be_hovered
            && (!self.check_mouse_mode || layout.mouse_mode.as_ref().unwrap().is_default())
            && layout.mouse_position.left() >= self.area.left
            && layout.mouse_position.top() >= self.area.top
            && layout.mouse_position.left() <= self.area.left + self.area.width
            && layout.mouse_position.top() <= self.area.top + self.area.height
            && layout.mouse_position.left() >= clip.left()
            && layout.mouse_position.left() <= clip.right()
            && layout.mouse_position.top() >= clip.top()
            && layout.mouse_position.top() <= clip.bottom();

        layout.can_be_hovered &= !(is_hovered && self.mark_after);

        is_hovered
    }
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

    clips: Vec<App::Clip>,
    active_clips: Vec<ClipId>,

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

            clips: vec![App::Clip::unbound()],
            active_clips: vec![ClipId(0)],

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

        self.clips.clear();
        self.clips.push(App::Clip::unbound());

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

    pub fn with_clip(&mut self, area: Area, mut f: impl FnMut(&mut Self)) {
        let clip = App::Clip::new(area.left, area.top, area.left + area.width, area.top + area.height);
        let parent_clip = self.clips[self.active_clips.last().unwrap().0];

        let combined_clip = combine_clip(clip, parent_clip);

        self.active_clips.push(ClipId(self.clips.len()));
        self.clips.push(combined_clip);

        f(self);

        self.active_clips.pop();
    }

    pub fn get_active_clip_id(&self) -> ClipId {
        self.active_clips.last().copied().unwrap()
    }

    pub fn with_secondary_background(&mut self, f: impl Fn(&mut Self)) -> bool {
        let previous = self.use_secondary_color;
        self.use_secondary_color = !self.use_secondary_color;

        f(self);

        self.use_secondary_color = previous;
        previous
    }

    pub fn register_click_handler(&mut self, button: MouseButton, handler: &'a dyn ClickHandler<App>) {
        self.layers[self.current_layer].click_handlers.push((button, handler));
    }

    pub fn register_drop_handler(&mut self, handler: &'a dyn DropHandler<App>) {
        self.layers[self.current_layer].drop_handlers.push(handler);
    }

    pub fn register_scroll_handler(&mut self, handler: &'a dyn ScrollHandler<App>) {
        self.layers[self.current_layer].scroll_handlers.push(handler);
    }

    pub fn register_input_handler(&mut self, input_handler: &'a dyn InputHandler<App>) {
        self.layers[self.current_layer].input_handlers.push(input_handler);
    }

    pub fn add_rectangle(
        &mut self,
        area: Area,
        corner_diameter: App::CornerDiameter,
        color: App::Color,
        shadow_color: App::Color,
        shadow_padding: App::ShadowPadding,
    ) {
        let clip_id = self.get_active_clip_id();
        let area = self.scale_area(area);
        let corner_diameter = corner_diameter.scaled(self.interface_scaling);
        let shadow_padding = shadow_padding.scaled(self.interface_scaling);

        self.layers[self.current_layer].rectangle_instructions.push(RectangleInstruction {
            clip_id,
            area,
            corner_diameter,
            color,
            shadow_color,
            shadow_padding,
        });
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_text(
        &mut self,
        area: Area,
        text: &'a str,
        font_size: App::FontSize,
        color: App::Color,
        highlight_color: App::Color,
        horizontal_alignment: HorizontalAlignment,
        vertical_alignment: VerticalAlignment,
        overflow_behavior: App::OverflowBehavior,
    ) {
        let clip_id = self.get_active_clip_id();
        let area = self.scale_area(area);
        let font_size = font_size.scaled(self.interface_scaling);
        let horizontal_alignment = horizontal_alignment.scaled(self.interface_scaling);
        let vertical_alignment = vertical_alignment.scaled(self.interface_scaling);

        self.layers[self.current_layer].text_instructions.push(TextInstruction {
            clip_id,
            area,
            text,
            font_size,
            color,
            highlight_color,
            horizontal_alignment,
            vertical_alignment,
            overflow_behavior,
        });
    }

    pub fn add_icon(&mut self, area: Area, icon: Icon<App>, color: App::Color) {
        let clip_id = self.get_active_clip_id();
        let area = self.scale_area(area);

        self.layers[self.current_layer].icon_instructions.push(IconInstruction {
            clip_id,
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

    #[cfg_attr(feature = "debug", korangar_debug::profile("render window layout"))]
    pub fn render(&mut self, renderer: &App::Renderer, text_layouter: &App::TextLayouter) {
        // NOTE: For now we scale the clip layers when rendering because we need the
        // unscaled clip for testing mouse interfection with areas. Ideally this
        // can be moved to `with_clip` at some point.
        for clip in &mut self.clips {
            *clip = App::Clip::new(
                (clip.left() - self.window_position.left()) * self.interface_scaling + self.window_position.left(),
                (clip.top() - self.window_position.top()) * self.interface_scaling + self.window_position.top(),
                (clip.right() - self.window_position.left()) * self.interface_scaling + self.window_position.left(),
                (clip.bottom() - self.window_position.top()) * self.interface_scaling + self.window_position.top(),
            );
        }

        for layer in self.layers.iter_mut() {
            #[cfg(feature = "debug")]
            korangar_debug::profile_block!("render layer");

            layer.rectangle_instructions.drain(..).for_each(
                |RectangleInstruction {
                     clip_id,
                     area,
                     corner_diameter,
                     color,
                     shadow_color,
                     shadow_padding,
                 }: RectangleInstruction<App>| {
                    #[cfg(feature = "debug")]
                    korangar_debug::profile_block!("rectangle instruction");

                    let clip = self.clips[clip_id.0];

                    renderer.render_rectangle(
                        App::Position::new(area.left, area.top),
                        App::Size::new(area.width, area.height),
                        clip,
                        corner_diameter,
                        color,
                        shadow_color,
                        shadow_padding,
                    );
                },
            );

            layer.icon_instructions.drain(..).for_each(
                |IconInstruction {
                     clip_id,
                     icon,
                     area,
                     color,
                 }: IconInstruction<App>| {
                    #[cfg(feature = "debug")]
                    korangar_debug::profile_block!("icon instruction");

                    let clip = self.clips[clip_id.0];

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

                renderer.render_custom(instruction, &self.clips);
            });

            layer.text_instructions.drain(..).for_each(
                |TextInstruction {
                     clip_id,
                     area,
                     text,
                     font_size,
                     color,
                     highlight_color,
                     horizontal_alignment,
                     vertical_alignment,
                     overflow_behavior,
                 }: TextInstruction<'_, App>| {
                    #[cfg(feature = "debug")]
                    korangar_debug::profile_block!("text instruction");

                    let clip = self.clips[clip_id.0];

                    let available_width = match horizontal_alignment {
                        HorizontalAlignment::Left { offset, border } => area.width - offset - border,
                        HorizontalAlignment::Center { border, .. } => area.width - border * 2.0,
                        HorizontalAlignment::Right { offset, border } => area.width - offset - border,
                    };

                    let (text_size, font_size) =
                        text_layouter.get_text_dimensions(text, color, highlight_color, font_size, available_width, overflow_behavior);

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
                        highlight_color,
                        font_size,
                    );
                },
            );
        }
    }

    pub fn handle_click(&self, state: &Context<App>, queue: &mut EventQueue<App>, mouse_button: MouseButton) {
        for layer in self.layers.iter().rev() {
            for (registered_button, click_handler) in &layer.click_handlers {
                if *registered_button == mouse_button {
                    click_handler.handle_click(state, queue);
                }
            }
        }
    }

    pub fn handle_drop(&self, state: &Context<App>, queue: &mut EventQueue<App>, mouse_mode: &'a MouseMode<App>) {
        for layer in self.layers.iter().rev() {
            for drop_handler in &layer.drop_handlers {
                drop_handler.handle_drop(state, queue, mouse_mode);
            }
        }
    }

    pub fn handle_scroll(&self, state: &Context<App>, queue: &mut EventQueue<App>, delta: f32) {
        for layer in self.layers.iter().rev() {
            for scroll_handler in &layer.scroll_handlers {
                if scroll_handler.handle_scroll(state, queue, delta) {
                    return;
                }
            }
        }
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
