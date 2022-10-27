use derive_new::new;
use num::Zero;

use crate::graphics::{Color, InterfaceRenderer, Renderer};
use crate::interface::{Element, *};

#[derive(new)]
pub struct Text {
    display: String,
    color: Color,
    font_size: f32,
    size_constraint: SizeConstraint,
    #[new(default)]
    state: ElementState,
}

impl Element for Text {
    fn get_state(&self) -> &ElementState {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState {
        &mut self.state
    }

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, _interface_settings: &InterfaceSettings, _theme: &Theme) {
        self.state.resolve(placement_resolver, &self.size_constraint);
    }

    fn is_focusable(&self) -> bool {
        false
    }

    fn render(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        renderer: &InterfaceRenderer,
        _state_provider: &StateProvider,
        interface_settings: &InterfaceSettings,
        _theme: &Theme,
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

        renderer.render_text(&self.display, Vector2::zero(), self.color, self.font_size);
    }
}
