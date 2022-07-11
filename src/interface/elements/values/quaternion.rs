use derive_new::new;
use num::Zero;
use std::fmt::Display;

use crate::types::maths::Quaternion;
use crate::graphics::Renderer;
use crate::interface::traits::Element;
use crate::interface::types::*;

#[derive(new)]
pub struct QuaternionValue<T: Display> {
    value: Quaternion<T>,
    #[new(default)]
    cached_values: String,
    #[new(value = "Size::zero()")]
    cached_size: Size,
    #[new(value = "Position::zero()")]
    cached_position: Position,
}

impl<T: Display> Element for QuaternionValue<T> {

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, _interface_settings: &InterfaceSettings, theme: &Theme) {
        let (size, position) = placement_resolver.allocate(&theme.value.size_constraint);
        self.cached_size = size.finalize();
        self.cached_position = position;
        self.cached_values = format!("{:.1}, {:.1}, {:.1} - {:.1}", self.value.v.x, self.value.v.y, self.value.v.z, self.value.s);
    }

    fn render(&self, renderer: &mut Renderer, _state_provider: &StateProvider, interface_settings: &InterfaceSettings, theme: &Theme, parent_position: Position, clip_size: Size, _hovered_element: Option<&dyn Element>, _second_theme: bool) {
        let absolute_position = parent_position + self.cached_position;
        let clip_size = clip_size.zip(absolute_position + self.cached_size, f32::min);
        renderer.render_rectangle(absolute_position, self.cached_size, clip_size, *theme.value.border_radius * *interface_settings.scaling, *theme.value.background_color);
        renderer.render_text(&self.cached_values, absolute_position + *theme.value.text_offset * *interface_settings.scaling, clip_size, *theme.value.foreground_color, *theme.value.font_size * *interface_settings.scaling);
    }
}
