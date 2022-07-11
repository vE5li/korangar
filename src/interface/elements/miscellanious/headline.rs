use derive_new::new;
use num::Zero;

use crate::interface::traits::Element;
use crate::interface::types::*;
use crate::graphics::Renderer;

#[derive(new)]
pub struct Headline {
    display: String,
    size_constraint: SizeConstraint,
    #[new(value = "Size::zero()")]
    cached_size: Size,
    #[new(value = "Position::zero()")]
    cached_position: Position,
}

impl Headline {

    pub const DEFAULT_SIZE: SizeConstraint = constraint!(100.0%, 12.0);
}

impl Element for Headline {

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, _interface_settings: &InterfaceSettings, _theme: &Theme) {
        let (size, position) = placement_resolver.allocate(&self.size_constraint);
        self.cached_size = size.finalize();
        self.cached_position = position;
    }

    fn render(&self, renderer: &mut Renderer, _state_provider: &StateProvider, interface_settings: &InterfaceSettings, theme: &Theme, parent_position: Position, clip_size: Size, _hovered_element: Option<&dyn Element>, _second_theme: bool) {
        let absolute_position = parent_position + self.cached_position;
        let clip_size = clip_size.zip(absolute_position + self.cached_size, f32::min);
        renderer.render_text(&self.display, absolute_position + *theme.label.text_offset * *interface_settings.scaling, clip_size, *theme.label.foreground_color, *theme.label.font_size * *interface_settings.scaling);
    }
}
