use derive_new::new;
use procedural::*;

use crate::graphics::{InterfaceRenderer, Renderer};
use crate::interface::{Element, *};

#[derive(new)]
pub struct Headline {
    display: String,
    size_constraint: SizeConstraint,
    #[new(default)]
    state: ElementState,
}

impl Headline {

    pub const DEFAULT_SIZE: SizeConstraint = constraint!(100%, 12);
}

impl Element for Headline {

    fn get_state(&self) -> &ElementState {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState {
        &mut self.state
    }

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, _interface_settings: &InterfaceSettings, _theme: &Theme) {
        self.state.resolve(placement_resolver, &self.size_constraint);
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
        _second_theme: bool,
    ) {

        let mut renderer = self
            .state
            .element_renderer(render_target, renderer, interface_settings, parent_position, clip_size);

        renderer.render_text(
            &self.display,
            *theme.label.text_offset,
            *theme.label.foreground_color,
            *theme.label.font_size,
        );
    }
}
