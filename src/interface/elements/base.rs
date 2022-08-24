use std::cell::RefCell;
use std::rc::Rc;

use cgmath::Vector2;

use crate::graphics::{InterfaceRenderer, Renderer};
use crate::interface::*;

pub type ElementCell = Rc<RefCell<dyn Element>>;

macro_rules! cell {
    ($element:expr) => {
        std::rc::Rc::new(std::cell::RefCell::new($element))
    };
}

pub trait Element {

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, interface_settings: &InterfaceSettings, theme: &Theme);

    fn update(&mut self) -> Option<ChangeEvent> {
        None
    }

    fn hovered_element(&self, _mouse_position: Vector2<f32>) -> HoverInformation {
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

    fn input_character(&mut self, _character: char) -> Option<ChangeEvent> {
        None
    }

    fn render(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        render: &InterfaceRenderer,
        state_provider: &StateProvider,
        interface_settings: &InterfaceSettings,
        theme: &Theme,
        parent_position: Position,
        clip_size: Size,
        hovered_element: Option<&dyn Element>,
        _focused_element: Option<&dyn Element>,
        second_theme: bool,
    );
}
