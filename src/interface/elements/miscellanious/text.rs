use derive_new::new;
use num::Zero;

use crate::interface::traits::Element;
use crate::interface::types::*;
use crate::graphics::{ Renderer, Color };

#[derive(new)]
pub struct Text {
    display: String,
    color: Color,
    font_size: f32,
    size_constraint: SizeConstraint,
    #[new(value = "Size::zero()")]
    cached_size: Size,
    #[new(value = "Position::zero()")]
    cached_position: Position,
}

impl Element for Text {

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, _interface_settings: &InterfaceSettings, _theme: &Theme) {
        let (size, position) = placement_resolver.allocate(&self.size_constraint);
        self.cached_size = size.finalize();
        self.cached_position = position;
    }

    fn render(&self, renderer: &mut Renderer, _state_provider: &StateProvider, interface_settings: &InterfaceSettings, _theme: &Theme, parent_position: Position, clip_size: Size, _hovered_element: Option<&dyn Element>, _second_theme: bool) {
        let absolute_position = parent_position + self.cached_position;
        let clip_size = vector2!(f32::min(clip_size.x, absolute_position.x + self.cached_size.x), f32::min(clip_size.y, absolute_position.y + self.cached_size.y));
        renderer.render_text(&self.display, absolute_position, clip_size, self.color, self.font_size * *interface_settings.scaling);
    }
}
