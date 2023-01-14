use derive_new::new;

use crate::graphics::{InterfaceRenderer, Renderer};
use crate::interface::{Element, *};

#[derive(new)]
pub struct StaticLabel {
    label: String,
    #[new(default)]
    state: ElementState,
}

impl Element for StaticLabel {
    fn get_state(&self) -> &ElementState {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState {
        &mut self.state
    }

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, interface_settings: &InterfaceSettings, theme: &Theme) {
        let mut size_constraint = theme.label.size_constraint;

        let size = placement_resolver.get_text_dimensions(
            &self.label,
            *theme.label.font_size,
            *theme.label.text_offset,
            *interface_settings.scaling,
            placement_resolver.get_available().x / 2.0, // TODO: make better
        );

        size_constraint.height = Dimension::Absolute(f32::max(size.y / *interface_settings.scaling, 14.0)); // TODO: make better

        self.state.resolve(placement_resolver, &size_constraint);
    }

    fn render(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        renderer: &InterfaceRenderer,
        _state_provider: &StateProvider,
        interface_settings: &InterfaceSettings,
        theme: &Theme,
        parent_position: Position,
        clip_size: ClipSize,
        _hovered_element: Option<&dyn Element>,
        _focused_element: Option<&dyn Element>,
        _mouse_mode: &MouseInputMode,
        _second_theme: bool,
    ) {
        let mut renderer = self
            .state
            .element_renderer(render_target, renderer, interface_settings, parent_position, clip_size);

        renderer.render_background(*theme.label.border_radius, *theme.label.background_color);

        renderer.render_text(
            &self.label,
            *theme.label.text_offset,
            *theme.label.foreground_color,
            *theme.label.font_size,
        );
    }
}
