use crate::graphics::{InterfaceRenderer, Renderer};
use crate::input::MouseInputMode;
use crate::interface::*;

#[derive(Default)]
pub struct CloseButton {
    state: ElementState,
}

impl Element for CloseButton {
    fn get_state(&self) -> &ElementState {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState {
        &mut self.state
    }

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, _interface_settings: &InterfaceSettings, theme: &InterfaceTheme) {
        let (size, position) = placement_resolver.allocate_right(&theme.close_button.size_constraint);
        self.state.cached_size = size.finalize();
        self.state.cached_position = position;
    }

    fn is_focusable(&self) -> bool {
        false
    }

    fn hovered_element(&self, mouse_position: ScreenPosition, mouse_mode: &MouseInputMode) -> HoverInformation {
        match mouse_mode {
            MouseInputMode::None => self.state.hovered_element(mouse_position),
            _ => HoverInformation::Missed,
        }
    }

    fn left_click(&mut self, _force_update: &mut bool) -> Vec<ClickAction> {
        vec![ClickAction::CloseWindow]
    }

    fn render(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        renderer: &InterfaceRenderer,
        _state_provider: &StateProvider,
        interface_settings: &InterfaceSettings,
        theme: &InterfaceTheme,
        parent_position: ScreenPosition,
        screen_clip: ScreenClip,
        hovered_element: Option<&dyn Element>,
        focused_element: Option<&dyn Element>,
        _mouse_mode: &MouseInputMode,
        _second_theme: bool,
    ) {
        let mut renderer = self
            .state
            .element_renderer(render_target, renderer, interface_settings, parent_position, screen_clip);

        let background_color = match self.is_element_self(hovered_element) || self.is_element_self(focused_element) {
            true => theme.close_button.hovered_background_color.get(),
            false => theme.close_button.background_color.get(),
        };

        renderer.render_background((theme.close_button.corner_radius.get()).into(), background_color);

        renderer.render_text(
            "X",
            theme.close_button.text_offset.get(),
            theme.close_button.foreground_color.get(),
            theme.close_button.font_size.get(),
        );
    }
}
