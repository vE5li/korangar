use types::maths::Vector2;
use graphics::Renderer;
use interface::types::{ PlacementResolver, StateProvider, Theme, HoverInformation, ClickAction, InterfaceSettings, Position, Size };

pub trait Element {

    fn update(&mut self, placement_resolver: &mut PlacementResolver, interface_settings: &InterfaceSettings, theme: &Theme);

    fn hovered_element(&self, _mouse_position: Vector2<f32>) -> HoverInformation {
        HoverInformation::Missed
    }

    fn left_click(&mut self, _update: &mut bool) -> Option<ClickAction> {
        None
    }

    fn right_click(&mut self, _update: &mut bool) -> Option<ClickAction> {
        None
    }

    fn drag(&mut self, _mouse_delta: Position, _update: &mut bool) -> bool {
        false 
    }

    fn render(&self, renderer: &mut Renderer, state_provider: &StateProvider, interface_settings: &InterfaceSettings, theme: &Theme, parent_position: Position, clip_size: Size, hovered_element: Option<&dyn Element>, second_theme: bool);
}
