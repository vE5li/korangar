use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use cgmath::{Vector2, Vector4, Zero};

use crate::graphics::{Color, InterfaceRenderer, Renderer, Texture};
use crate::input::MouseInputMode;
use crate::interface::*;
use crate::inventory::Item;

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
    render_target: &'a mut <InterfaceRenderer as Renderer>::Target,
    renderer: &'a InterfaceRenderer,
    interface_settings: &'a InterfaceSettings,
    position: Position,
    size: Size,
    clip_size: ClipSize,
}

impl<'a> ElementRenderer<'a> {
    pub fn get_text_dimensions(&self, text: &str, font_size: f32, available_width: f32) -> Vector2<f32> {
        self.renderer
            .get_text_dimensions(text, font_size * *self.interface_settings.scaling, available_width)
    }

    pub fn set_scroll(&mut self, scroll: f32) {
        self.position.y -= scroll;
    }

    pub fn render_background(&mut self, border_radius: Vector4<f32>, color: Color) {
        self.renderer.render_rectangle(
            self.render_target,
            self.position,
            self.size,
            self.clip_size,
            border_radius * *self.interface_settings.scaling,
            color,
        );
    }

    pub fn render_rectangle(&mut self, position: Position, size: Size, border_radius: Vector4<f32>, color: Color) {
        self.renderer.render_rectangle(
            self.render_target,
            self.position + position,
            size,
            self.clip_size,
            border_radius * *self.interface_settings.scaling,
            color,
        );
    }

    pub fn render_text(&mut self, text: &str, offset: Position, foreground_color: Color, font_size: f32) -> f32 {
        self.renderer.render_text(
            self.render_target,
            text,
            self.position + offset * *self.interface_settings.scaling,
            self.clip_size,
            foreground_color,
            font_size * *self.interface_settings.scaling,
        )
    }

    pub fn render_checkbox(&mut self, offset: Position, size: Size, color: Color, checked: bool) {
        self.renderer.render_checkbox(
            self.render_target,
            self.position + offset * *self.interface_settings.scaling,
            size * *self.interface_settings.scaling,
            self.clip_size,
            color,
            checked,
        );
    }

    pub fn render_expand_arrow(&mut self, offset: Position, size: Size, color: Color, expanded: bool) {
        self.renderer.render_expand_arrow(
            self.render_target,
            self.position + offset * *self.interface_settings.scaling,
            size * *self.interface_settings.scaling,
            self.clip_size,
            color,
            expanded,
        );
    }

    pub fn render_sprite(&mut self, texture: Texture, offset: Position, size: Size, color: Color) {
        self.renderer.render_sprite(
            self.render_target,
            texture,
            self.position + offset * *self.interface_settings.scaling,
            size * *self.interface_settings.scaling,
            self.clip_size,
            color,
            false,
        );
    }

    pub fn render_element(
        &mut self,
        element: &dyn Element,
        state_provider: &StateProvider,
        interface_settings: &InterfaceSettings,
        theme: &Theme,
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
            self.clip_size,
            hovered_element,
            focused_element,
            mouse_mode,
            second_theme,
        )
    }
}

pub struct ElementState {
    pub cached_size: Size,
    pub cached_position: Position,
    pub parent_element: Option<Weak<RefCell<dyn Element>>>,
    pub mouse_position: Cell<Position>,
}

impl Default for ElementState {
    fn default() -> Self {
        Self {
            cached_size: Size::zero(),
            cached_position: Position::zero(),
            parent_element: None,
            mouse_position: Cell::new(Position::zero()),
        }
    }
}

impl ElementState {
    pub fn link_back(&mut self, weak_parent: Option<Weak<RefCell<dyn Element>>>) {
        self.parent_element = weak_parent;
    }

    pub fn resolve(&mut self, placement_resolver: &mut PlacementResolver, size_constraint: &SizeConstraint) {
        let (size, position) = placement_resolver.allocate(size_constraint);
        self.cached_size = size.finalize();
        self.cached_position = position;
    }

    pub fn hovered_element(&self, mouse_position: Position) -> HoverInformation {
        let absolute_position = mouse_position - self.cached_position;

        if absolute_position.x >= 0.0
            && absolute_position.y >= 0.0
            && absolute_position.x <= self.cached_size.x
            && absolute_position.y <= self.cached_size.y
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
        parent_position: Position,
        clip_size: ClipSize,
    ) -> ElementRenderer<'a> {
        let position = parent_position + self.cached_position;
        let size = self.cached_size;

        let clip_size = Vector4::new(
            clip_size.x.max(position.x),
            clip_size.y.max(position.y),
            clip_size.z.min(position.x + self.cached_size.x),
            clip_size.w.min(position.y + self.cached_size.y),
        );

        ElementRenderer {
            render_target,
            renderer,
            interface_settings,
            position,
            size,
            clip_size,
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

    fn link_back(&mut self, _weak_self: Weak<RefCell<dyn Element>>, weak_parent: Option<Weak<RefCell<dyn Element>>>) {
        self.get_state_mut().link_back(weak_parent);
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

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, interface_settings: &InterfaceSettings, theme: &Theme);

    fn update(&mut self) -> Option<ChangeEvent> {
        None
    }

    fn is_element_self(&self, element: Option<&dyn Element>) -> bool {
        matches!(element, Some(reference) if std::ptr::eq(reference as *const _ as *const (), self as *const _ as *const ()))
    }

    fn hovered_element(&self, _mouse_position: Vector2<f32>, _mouse_mode: &MouseInputMode) -> HoverInformation {
        HoverInformation::Missed
    }

    fn left_click(&mut self, _update: &mut bool) -> Option<ClickAction> {
        None
    }

    fn right_click(&mut self, _update: &mut bool) -> Option<ClickAction> {
        None
    }

    fn drag(&mut self, _mouse_delta: Position) -> Option<ChangeEvent> {
        None
    }

    fn input_character(&mut self, _character: char) -> Option<ClickAction> {
        None
    }

    fn drop_item(&mut self, _item_source: ItemSource, _item: Item) -> Option<ItemMove> {
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
        theme: &Theme,
        parent_position: Position,
        clip_size: ClipSize,
        hovered_element: Option<&dyn Element>,
        focused_element: Option<&dyn Element>,
        mouse_mode: &MouseInputMode,
        second_theme: bool,
    );
}
