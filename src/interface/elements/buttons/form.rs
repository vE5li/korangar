use derive_new::new;
use num::Zero;

use crate::graphics::{InterfaceRenderer, Renderer};
use crate::input::UserEvent;
use crate::interface::*;

#[derive(new)]
pub struct FormButton {
    text: &'static str,
    selector: Box<dyn Fn() -> bool>,
    action: Box<dyn Fn() -> UserEvent>,
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

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, _interface_settings: &InterfaceSettings, theme: &Theme) {
        self.state.resolve(placement_resolver, &theme.button.size_constraint);
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
        _focused_element: Option<&dyn Element>,
        _second_theme: bool,
    ) {

        let mut renderer = self
            .state
            .element_renderer(render_target, renderer, interface_settings, parent_position, clip_size);

        //let background_color = theme.button.background.choose(self.is_element_self(hovered_element), self.is_element_self(focused_element));
        let background_color = match self.is_element_self(hovered_element) {
            true => *theme.button.hovered_background_color,
            false => *theme.button.background_color,
        };

        renderer.render_background(*theme.button.border_radius, background_color);

        renderer.render_checkbox(
            *theme.button.icon_offset,
            *theme.button.icon_size,
            *theme.button.foreground_color,
            (self.selector)(),
        );

        renderer.render_text(
            self.text,
            *theme.button.icon_text_offset,
            *theme.button.foreground_color,
            *theme.button.font_size,
        );
    }
}
