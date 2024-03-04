use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::sync::Arc;

use cgmath::Vector2;
use vulkano::image::view::ImageView;

use crate::graphics::{Color, InterfaceRenderer, Renderer, SpriteRenderer};
use crate::input::MouseInputMode;
use crate::interface::*;
use crate::inventory::{Item, Skill};

pub type ElementCell = Rc<RefCell<dyn Element>>;
pub type WeakElementCell = Weak<RefCell<dyn Element>>;

pub trait ElementWrap {
    fn wrap(self) -> ElementCell;
}

impl<T> ElementWrap for T
where
    T: Element + Sized + 'static,
{
    fn wrap(self) -> ElementCell {
        Rc::new(RefCell::new(self))
    }
}

pub struct ElementRenderer<'a> {
    pub render_target: &'a mut <InterfaceRenderer as Renderer>::Target,
    pub renderer: &'a InterfaceRenderer,
    pub interface_settings: &'a InterfaceSettings,
    pub position: ScreenPosition,
    pub size: ScreenSize,
    pub screen_clip: ScreenClip,
}

impl<'a> ElementRenderer<'a> {
    pub fn get_position(&self) -> ScreenPosition {
        self.position
    }

    pub fn get_text_dimensions(&self, text: &str, font_size: f32, available_width: f32) -> Vector2<f32> {
        self.renderer
            .get_text_dimensions(text, font_size * self.interface_settings.scaling.get(), available_width)
    }

    pub fn set_scroll(&mut self, scroll: f32) {
        self.position.top -= scroll;
    }

    pub fn render_background(&mut self, corner_radius: CornerRadius, color: Color) {
        self.renderer.render_rectangle(
            self.render_target,
            self.position,
            self.size,
            self.screen_clip,
            corner_radius * self.interface_settings.scaling.get(),
            color,
        );
    }

    pub fn render_rectangle(&mut self, position: ScreenPosition, size: ScreenSize, corner_radius: CornerRadius, color: Color) {
        self.renderer.render_rectangle(
            self.render_target,
            self.position + position,
            size,
            self.screen_clip,
            corner_radius * self.interface_settings.scaling.get(),
            color,
        );
    }

    pub fn render_text(&mut self, text: &str, offset: ScreenPosition, foreground_color: Color, font_size: f32) -> f32 {
        self.renderer.render_text(
            self.render_target,
            text,
            self.position + offset * self.interface_settings.scaling.get(),
            self.screen_clip,
            foreground_color,
            font_size * self.interface_settings.scaling.get(),
        )
    }

    pub fn render_checkbox(&mut self, offset: ScreenPosition, size: ScreenSize, color: Color, checked: bool) {
        self.renderer.render_checkbox(
            self.render_target,
            self.position + offset * self.interface_settings.scaling.get(),
            size * self.interface_settings.scaling.get(),
            self.screen_clip,
            color,
            checked,
        );
    }

    pub fn render_expand_arrow(&mut self, offset: ScreenPosition, size: ScreenSize, color: Color, expanded: bool) {
        self.renderer.render_expand_arrow(
            self.render_target,
            self.position + offset * self.interface_settings.scaling.get(),
            size * self.interface_settings.scaling.get(),
            self.screen_clip,
            color,
            expanded,
        );
    }

    pub fn render_sprite(&mut self, texture: Arc<ImageView>, offset: ScreenPosition, size: ScreenSize, color: Color) {
        self.renderer.render_sprite(
            self.render_target,
            texture,
            self.position + offset * self.interface_settings.scaling.get(),
            size * self.interface_settings.scaling.get(),
            self.screen_clip,
            color,
            false,
        );
    }

    pub fn render_element(
        &mut self,
        element: &dyn Element,
        state_provider: &StateProvider,
        interface_settings: &InterfaceSettings,
        theme: &InterfaceTheme,
        hovered_element: Option<&dyn Element>,
        focused_element: Option<&dyn Element>,
        mouse_mode: &MouseInputMode,
        second_theme: bool,
    ) {
        element.render(
            self.render_target,
            self.renderer,
            state_provider,
            interface_settings,
            theme,
            self.position,
            self.screen_clip,
            hovered_element,
            focused_element,
            mouse_mode,
            second_theme,
        )
    }
}

#[derive(Default)]
pub struct ElementState {
    pub cached_size: ScreenSize,
    pub cached_position: ScreenPosition,
    pub self_element: Option<WeakElementCell>,
    pub parent_element: Option<WeakElementCell>,
    pub mouse_position: Cell<ScreenPosition>,
}

impl ElementState {
    pub fn link_back(&mut self, weak_self: WeakElementCell, weak_parent: Option<WeakElementCell>) {
        self.self_element = Some(weak_self);
        self.parent_element = weak_parent;
    }

    pub fn resolve(&mut self, placement_resolver: &mut PlacementResolver, size_bound: &SizeBound) {
        let (size, position) = placement_resolver.allocate(size_bound);
        self.cached_size = size.finalize();
        self.cached_position = position;
    }

    pub fn hovered_element(&self, mouse_position: ScreenPosition) -> HoverInformation {
        let absolute_position = ScreenPosition::from_size(mouse_position - self.cached_position);

        if absolute_position.left >= 0.0
            && absolute_position.top >= 0.0
            && absolute_position.left <= self.cached_size.width
            && absolute_position.top <= self.cached_size.height
        {
            self.mouse_position.replace(absolute_position);
            return HoverInformation::Hovered;
        }

        HoverInformation::Missed
    }

    pub fn element_renderer<'a>(
        &self,
        render_target: &'a mut <InterfaceRenderer as Renderer>::Target,
        renderer: &'a InterfaceRenderer,
        interface_settings: &'a InterfaceSettings,
        parent_position: ScreenPosition,
        screen_clip: ScreenClip,
    ) -> ElementRenderer<'a> {
        let position = parent_position + self.cached_position;
        let size = self.cached_size;

        let screen_clip = ScreenClip {
            left: screen_clip.left.max(position.left),
            top: screen_clip.top.max(position.top),
            right: screen_clip.right.min(position.left + self.cached_size.width),
            bottom: screen_clip.bottom.min(position.top + self.cached_size.height),
        };

        ElementRenderer {
            render_target,
            renderer,
            interface_settings,
            position,
            size,
            screen_clip,
        }
    }
}

#[derive(Clone, Copy, new)]
pub struct Focus {
    pub mode: FocusMode,
    #[new(default)]
    pub downwards: bool,
}

impl Focus {
    pub fn downwards() -> Self {
        Self {
            mode: FocusMode::FocusNext,
            downwards: true,
        }
    }

    pub fn to_downwards(self) -> Self {
        Focus {
            mode: self.mode,
            downwards: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FocusMode {
    FocusNext,
    FocusPrevious,
}

impl From<bool> for FocusMode {
    fn from(reverse: bool) -> Self {
        match reverse {
            true => Self::FocusPrevious,
            false => Self::FocusNext,
        }
    }
}

pub trait Element {
    fn get_state(&self) -> &ElementState;

    fn get_state_mut(&mut self) -> &mut ElementState;

    fn link_back(&mut self, weak_self: WeakElementCell, weak_parent: Option<WeakElementCell>) {
        self.get_state_mut().link_back(weak_self, weak_parent);
    }

    fn is_focusable(&self) -> bool {
        true
    }

    fn focus_next(
        &self,
        self_cell: Rc<RefCell<dyn Element>>,
        _caller_cell: Option<Rc<RefCell<dyn Element>>>,
        focus: Focus,
    ) -> Option<Rc<RefCell<dyn Element>>> {
        if focus.downwards {
            return Some(self_cell);
        }

        self.get_state().parent_element.as_ref().and_then(|parent_element| {
            let parent_element = parent_element.upgrade().unwrap();
            let next_element = parent_element.borrow().focus_next(parent_element.clone(), Some(self_cell), focus);
            next_element
        })
    }

    fn restore_focus(&self, self_cell: ElementCell) -> Option<ElementCell> {
        self.is_focusable().then_some(self_cell)
    }

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, interface_settings: &InterfaceSettings, theme: &InterfaceTheme);

    fn update(&mut self) -> Option<ChangeEvent> {
        None
    }

    fn is_element_self(&self, element: Option<&dyn Element>) -> bool {
        matches!(element, Some(reference) if std::ptr::eq(reference as *const _ as *const (), self as *const _ as *const ()))
    }

    fn hovered_element(&self, _mouse_position: ScreenPosition, _mouse_mode: &MouseInputMode) -> HoverInformation {
        HoverInformation::Missed
    }

    fn left_click(&mut self, _update: &mut bool) -> Vec<ClickAction> {
        Vec::new()
    }

    fn right_click(&mut self, _update: &mut bool) -> Vec<ClickAction> {
        Vec::new()
    }

    fn drag(&mut self, _mouse_delta: ScreenPosition) -> Option<ChangeEvent> {
        None
    }

    fn input_character(&mut self, _character: char) -> Vec<ClickAction> {
        Vec::new()
    }

    fn drop_item(&mut self, _item_source: ItemSource, _item: Item) -> Option<ItemMove> {
        None
    }

    fn drop_skill(&mut self, skill_source: SkillSource, skill: Skill) -> Option<SkillMove> {
        let _ = skill_source;
        let _ = skill;
        None
    }

    fn scroll(&mut self, delta: f32) -> Option<ChangeEvent> {
        self.get_state()
            .parent_element
            .as_ref()
            .and_then(|weak_pointer| weak_pointer.upgrade())
            .and_then(|element| (*element).borrow_mut().scroll(delta))
    }

    fn render(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        render: &InterfaceRenderer,
        state_provider: &StateProvider,
        interface_settings: &InterfaceSettings,
        theme: &InterfaceTheme,
        parent_position: ScreenPosition,
        screen_clip: ScreenClip,
        hovered_element: Option<&dyn Element>,
        focused_element: Option<&dyn Element>,
        mouse_mode: &MouseInputMode,
        second_theme: bool,
    );
}
