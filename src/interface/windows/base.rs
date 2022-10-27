use cgmath::Vector2;

use crate::graphics::{InterfaceRenderer, Renderer};
use crate::input::MouseInputMode;
use crate::interface::{Element, *};

pub trait Window {
    fn get_window_class(&self) -> Option<&str>;

    fn has_transparency(&self, theme: &Theme) -> bool;

    fn resolve(
        &mut self,
        interface_settings: &InterfaceSettings,
        theme: &Theme,
        avalible_space: Size,
    ) -> (Option<&str>, Vector2<f32>, Size);

    fn update(&mut self) -> Option<ChangeEvent>;

    fn first_focused_element(&self) -> Option<ElementCell>;

    fn restore_focus(&self) -> Option<ElementCell>;

    fn hovered_element(&self, mouse_position: Vector2<f32>, mouse_mode: &MouseInputMode) -> HoverInformation;

    fn get_area(&self) -> (Position, Size);

    fn hovers_area(&self, position: Position, size: Size) -> bool;

    fn offset(&mut self, avalible_space: Size, offset: Vector2<f32>) -> Option<(&str, Vector2<f32>)>;

    fn validate_position(&mut self, avalible_space: Size);

    fn resize(
        &mut self,
        interface_settings: &InterfaceSettings,
        theme: &Theme,
        avalible_space: Size,
        growth: Vector2<f32>,
    ) -> (Option<&str>, Size);

    fn validate_size(&mut self, interface_settings: &InterfaceSettings, avalible_space: Size);

    fn render(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        renderer: &InterfaceRenderer,
        state_provider: &StateProvider,
        interface_settings: &InterfaceSettings,
        theme: &Theme,
        hovered_element: Option<&dyn Element>,
        focused_element: Option<&dyn Element>,
        mouse_mode: &MouseInputMode,
    );
}
