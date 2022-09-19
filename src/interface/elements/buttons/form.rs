use derive_new::new;

use crate::graphics::{InterfaceRenderer, Renderer};
use crate::input::UserEvent;
use crate::interface::*;

#[derive(new)]
pub struct FormButton {
    text: &'static str,
    selector: Box<dyn Fn() -> bool>,
    action: Box<dyn Fn() -> UserEvent>,
    width_constraint: DimensionConstraint,
    #[new(default)]
    state: ElementState,
}

impl Element for FormButton {

    fn get_state(&self) -> &ElementState {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState {
        &mut self.state
    }

    fn is_focusable(&self) -> bool {
        (self.selector)()
    }

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, _interface_settings: &InterfaceSettings, theme: &Theme) {

        let size_constraint = theme.button.height_constraint.add_width(self.width_constraint);
        self.state.resolve(placement_resolver, &size_constraint);
    }

    fn hovered_element(&self, mouse_position: Position) -> HoverInformation {
        self.state.hovered_element(mouse_position)
    }

    fn left_click(&mut self, _force_update: &mut bool) -> Option<ClickAction> {
        (self.selector)().then(|| (self.action)()).map(ClickAction::Event)
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
        hovered_element: Option<&dyn Element>,
        focused_element: Option<&dyn Element>,
        _second_theme: bool,
    ) {

        let mut renderer = self
            .state
            .element_renderer(render_target, renderer, interface_settings, parent_position, clip_size);

        let disabled = !(self.selector)();

        //let background_color = theme.button.background.choose(self.is_element_self(hovered_element), self.is_element_self(focused_element));
        let background_color = match self.is_element_self(hovered_element) || self.is_element_self(focused_element) {
            _ if disabled => *theme.button.disabled_background_color,
            true => *theme.button.hovered_background_color,
            false => *theme.button.background_color,
        };

        renderer.render_background(*theme.button.border_radius, background_color);

        let foreground_color = match disabled {
            true => *theme.button.disabled_foreground_color,
            false => *theme.button.foreground_color,
        };

        renderer.render_text(self.text, *theme.button.text_offset, foreground_color, *theme.button.font_size);
    }
}
