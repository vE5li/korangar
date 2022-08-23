use derive_new::new;
use num::Zero;

use crate::interface::Element;
use crate::interface::*;
use crate::graphics::{ Renderer, Color, InterfaceRenderer };

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

    fn render(&self, render_target: &mut <InterfaceRenderer as Renderer>::Target, renderer: &InterfaceRenderer, _state_provider: &StateProvider, interface_settings: &InterfaceSettings, _theme: &Theme, parent_position: Position, clip_size: Size, _hovered_element: Option<&dyn Element>, _focused_element: Option<&dyn Element>, _second_theme: bool) {
        let absolute_position = parent_position + self.cached_position;
        let clip_size = clip_size.zip(absolute_position + self.cached_size, f32::min);
        renderer.render_text(render_target, &self.display, absolute_position, clip_size, self.color, self.font_size * *interface_settings.scaling);
    }
}
