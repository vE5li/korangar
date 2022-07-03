use derive_new::new;
use num::Zero;

use crate::interface::traits::Element;
use crate::interface::types::*;
use crate::graphics::Renderer;

#[derive(new)]
pub struct StaticLabel {
    label: String,
    #[new(value = "Size::zero()")]
    cached_size: Size,
    #[new(value = "Position::zero()")]
    cached_position: Position,
}

impl Element for StaticLabel {

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, interface_settings: &InterfaceSettings, theme: &Theme) {

        let mut size_constraint = theme.label.size_constraint;
        let width = self.label.len() as f32 * 8.0 + theme.label.text_offset.x * *interface_settings.scaling * 2.0;
        size_constraint.width = Dimension::Absolute(width);

        let (size, position) = placement_resolver.allocate(&size_constraint);
        self.cached_size = size.finalize();
        self.cached_position = position;
    }

    fn render(&self, renderer: &mut Renderer, _state_provider: &StateProvider, interface_settings: &InterfaceSettings, theme: &Theme, parent_position: Position, clip_size: Size, _hovered_element: Option<&dyn Element>, _second_theme: bool) {
        let absolute_position = parent_position + self.cached_position;
        let clip_size = vector2!(f32::min(clip_size.x, absolute_position.x + self.cached_size.x), f32::min(clip_size.y, absolute_position.y + self.cached_size.y));

        renderer.render_rectangle(absolute_position, self.cached_size, clip_size, *theme.label.border_radius * *interface_settings.scaling, *theme.label.background_color);
        renderer.render_text(&self.label, absolute_position + *theme.label.text_offset * *interface_settings.scaling, clip_size, *theme.label.foreground_color, *theme.label.font_size * *interface_settings.scaling);
    }
}
