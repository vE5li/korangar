use derive_new::new;
use num::Zero;
use std::fmt::Display;

use types::maths::Vector2;
use graphics::Renderer;
use interface::traits::Element;
use interface::types::*;

#[derive(new)]
pub struct Vector2Value<T: Display> {
    value: Vector2<T>,
    #[new(default)]
    cached_values: String,
    #[new(value = "Size::zero()")]
    cached_size: Size,
    #[new(value = "Position::zero()")]
    cached_position: Position,
}

impl<T: Display> Element for Vector2Value<T> {

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, _interface_settings: &InterfaceSettings, theme: &Theme) {
        let (size, position) = placement_resolver.allocate(&theme.value.size_constraint);
        self.cached_size = size.finalize();
        self.cached_position = position;
        self.cached_values = format!("{:.1}, {:.1}", self.value.x, self.value.y);
    }

    fn render(&self, renderer: &mut Renderer, _state_provider: &StateProvider, interface_settings: &InterfaceSettings, theme: &Theme, parent_position: Position, clip_size: Size, _hovered_element: Option<&dyn Element>, _second_theme: bool) {
        let absolute_position = parent_position + self.cached_position;
        let clip_size = vector2!(f32::min(clip_size.x, absolute_position.x + self.cached_size.x), f32::min(clip_size.y, absolute_position.y + self.cached_size.y));
        renderer.render_rectangle(absolute_position, self.cached_size, clip_size, *theme.value.border_radius * *interface_settings.scaling, *theme.value.background_color);
        renderer.render_text(&self.cached_values, absolute_position + *theme.value.text_offset * *interface_settings.scaling, clip_size, *theme.value.foreground_color, *theme.value.font_size * *interface_settings.scaling);
    }
}
