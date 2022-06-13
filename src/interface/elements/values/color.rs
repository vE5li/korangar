use derive_new::new;
use num::Zero;

use graphics::{ Renderer, Color };
use interface::traits::Element;
use interface::types::*;

#[derive(new)]
pub struct ColorValue {
    color: Color,
    #[new(default)]
    cached_values: String,
    #[new(value = "Size::zero()")]
    cached_size: Size,
    #[new(value = "Position::zero()")]
    cached_position: Position,
}

impl Element for ColorValue {

    fn update(&mut self, placement_resolver: &mut PlacementResolver, _interface_settings: &InterfaceSettings, theme: &Theme) {
        let (size, position) = placement_resolver.allocate(&theme.value.size_constraint);
        self.cached_size = size.finalize();
        self.cached_position = position;
        self.cached_values = format!("{}, {}, {}", self.color.red, self.color.green, self.color.blue);
    }

    fn render(&self, renderer: &mut Renderer, _state_provider: &StateProvider, interface_settings: &InterfaceSettings, theme: &Theme, parent_position: Position, clip_size: Size, _hovered_element: Option<&dyn Element>, _second_theme: bool) {
        let absolute_position = parent_position + self.cached_position;
        let clip_size = vector2!(f32::min(clip_size.x, absolute_position.x + self.cached_size.x), f32::min(clip_size.y, absolute_position.y + self.cached_size.y));
        renderer.render_rectangle(absolute_position, self.cached_size, clip_size, *theme.value.border_radius * *interface_settings.scaling, self.color);
        renderer.render_text(&self.cached_values, absolute_position + *theme.value.text_offset * *interface_settings.scaling, clip_size, self.color.invert(), *theme.value.font_size * *interface_settings.scaling);
    }
}