use crate::graphics::{Color, InterfaceRenderer, Renderer};
use crate::interface::{Element, *};

pub struct ColorValue {
    color: Color,
    display: String,
    state: ElementState,
}

impl ColorValue {

    pub fn new(color: Color) -> Self {

        let display = format!("{}, {}, {}, {}", color.red, color.green, color.blue, color.alpha);
        let state = ElementState::default();

        Self { color, display, state }
    }
}

impl Element for ColorValue {

    fn get_state(&self) -> &ElementState {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState {
        &mut self.state
    }

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, _interface_settings: &InterfaceSettings, theme: &Theme) {
        self.state.resolve(placement_resolver, &theme.value.size_constraint);
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

        renderer.render_background(*theme.value.border_radius, self.color);

        renderer.render_text(
            &self.display,
            *theme.value.text_offset,
            self.color.invert(),
            *theme.value.font_size,
        );
    }
}
