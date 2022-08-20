use derive_new::new;
use num::Zero;

use crate::graphics::{ Renderer, Color, InterfaceRenderer };
use crate::interface::traits::Element;
use crate::interface::types::*;

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

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, _interface_settings: &InterfaceSettings, theme: &Theme) {
        let (size, position) = placement_resolver.allocate(&theme.value.size_constraint);
        self.cached_size = size.finalize();
        self.cached_position = position;
        self.cached_values = format!("{}, {}, {}, {}", self.color.red, self.color.green, self.color.blue, self.color.alpha);
    }

    fn render(&self, render_target: &mut <InterfaceRenderer as Renderer>::Target, renderer: &InterfaceRenderer, _state_provider: &StateProvider, interface_settings: &InterfaceSettings, theme: &Theme, parent_position: Position, clip_size: Size, _hovered_element: Option<&dyn Element>, _focused_element: Option<&dyn Element>, _second_theme: bool) {
        let absolute_position = parent_position + self.cached_position;
        let clip_size = clip_size.zip(absolute_position + self.cached_size, f32::min);
        renderer.render_rectangle(render_target, absolute_position, self.cached_size, clip_size, *theme.value.border_radius * *interface_settings.scaling, self.color);
        renderer.render_text(render_target, &self.cached_values, absolute_position + *theme.value.text_offset * *interface_settings.scaling, clip_size, self.color.invert(), *theme.value.font_size * *interface_settings.scaling);
    }
}
