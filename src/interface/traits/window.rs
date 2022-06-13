use types::maths::Vector2;
use graphics::Renderer;
use interface::types::{ StateProvider, Theme, HoverInformation, Position, Size, InterfaceSettings };
use interface::traits::Element;

pub trait Window {

    fn identifier_matches(&self, identifier: &str) -> bool;

    fn update(&mut self, interface_settings: &InterfaceSettings, theme: &Theme, avalible_space: Size) -> (Option<&str>, Vector2<f32>, Size);

    fn hovered_element(&self, mouse_position: Vector2<f32>) -> HoverInformation;

    fn get_area(&self) -> (Position, Size);

    fn hovers_area(&self, position: Position, size: Size) -> bool;

    fn offset(&mut self, offset: Vector2<f32>) -> Option<(&str, Vector2<f32>)>;

    fn resize(&mut self, interface_settings: &InterfaceSettings, theme: &Theme, avalible_space: Size, growth: Vector2<f32>) -> (Option<&str>, Size);

    fn validate_size(&mut self, interface_settings: &InterfaceSettings, avalible_space: Size);

    fn render(&self, renderer: &mut Renderer, state_provider: &StateProvider, interface_settings: &InterfaceSettings, theme: &Theme, hovered_element: Option<&dyn Element>);
}

