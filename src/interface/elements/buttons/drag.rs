use derive_new::new;

use crate::graphics::{InterfaceRenderer, Renderer};
use crate::input::MouseInputMode;
use crate::interface::{Element, *};

#[derive(new)]
pub struct DragButton {
    window_title: String,
    width_constraint: DimensionConstraint,
    #[new(default)]
    state: ElementState,
}

impl Element for DragButton {
    fn get_state(&self) -> &ElementState {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState {
        &mut self.state
    }

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, _interface_settings: &InterfaceSettings, theme: &InterfaceTheme) {
        let size_constraint = self.width_constraint.add_height(theme.window.title_height);

        self.state.resolve(placement_resolver, &size_constraint);
    }

    fn is_focusable(&self) -> bool {
        false
    }

    fn hovered_element(&self, mouse_position: ScreenPosition, mouse_mode: &MouseInputMode) -> HoverInformation {
        match mouse_mode {
            MouseInputMode::None => self.state.hovered_element(mouse_position),
            MouseInputMode::DragElement((element, _)) if self.is_element_self(Some(&*element.borrow())) => HoverInformation::Hovered,
            _ => HoverInformation::Missed,
        }
    }

    fn left_click(&mut self, _force_update: &mut bool) -> Vec<ClickAction> {
        vec![ClickAction::MoveInterface]
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
        _focused_element: Option<&dyn Element>,
        _mouse_mode: &MouseInputMode,
        _second_theme: bool,
    ) {
        let mut renderer = self
            .state
            .element_renderer(render_target, renderer, interface_settings, parent_position, screen_clip);

        if self.is_element_self(hovered_element) {
            renderer.render_background((*theme.window.title_corner_radius).into(), *theme.window.title_background_color);
        }

        let text_position = ScreenPosition {
            left: theme.window.text_offset.x,
            top: theme.window.text_offset.y,
        };

        renderer.render_text(
            &self.window_title,
            text_position,
            *theme.window.foreground_color,
            *theme.window.font_size,
        );
    }
}
