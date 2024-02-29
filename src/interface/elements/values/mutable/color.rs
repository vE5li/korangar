use crate::graphics::{Color, InterfaceRenderer, Renderer};
use crate::input::MouseInputMode;
use crate::interface::{ColorWindow, Element, *};

pub struct MutableColorValue {
    name: String,
    color_pointer: *const Color,
    change_event: Option<ChangeEvent>,
    cached_color: Color,
    cached_values: String,
    state: ElementState,
}

impl MutableColorValue {
    pub fn new(name: String, color_pointer: *const Color, change_event: Option<ChangeEvent>) -> Self {
        let cached_color = unsafe { *color_pointer };
        let cached_values = format!(
            "{}, {}, {}, {}",
            cached_color.red, cached_color.green, cached_color.blue, cached_color.alpha
        );
        let state = ElementState::default();

        Self {
            name,
            color_pointer,
            change_event,
            cached_color,
            cached_values,
            state,
        }
    }
}

impl Element for MutableColorValue {
    fn get_state(&self) -> &ElementState {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState {
        &mut self.state
    }

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, _interface_settings: &InterfaceSettings, theme: &InterfaceTheme) {
        self.state.resolve(placement_resolver, &theme.value.size_constraint);
    }

    fn update(&mut self) -> Option<ChangeEvent> {
        let current_color = unsafe { *self.color_pointer };

        if self.cached_color != current_color {
            self.cached_color = current_color;
            self.cached_values = format!(
                "{}, {}, {}, {}",
                self.cached_color.red, self.cached_color.green, self.cached_color.blue, self.cached_color.alpha
            );
            return Some(ChangeEvent::RENDER_WINDOW);
        }

        None
    }

    fn hovered_element(&self, mouse_position: ScreenPosition, mouse_mode: &MouseInputMode) -> HoverInformation {
        match mouse_mode {
            MouseInputMode::None => self.state.hovered_element(mouse_position),
            _ => HoverInformation::Missed,
        }
    }

    fn left_click(&mut self, _force_update: &mut bool) -> Vec<ClickAction> {
        vec![ClickAction::OpenWindow(Box::new(ColorWindow::new(
            self.name.clone(),
            self.color_pointer,
            self.change_event,
        )))]
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

        let background_color = match self.is_element_self(hovered_element) {
            true => self.cached_color.shade(),
            false => self.cached_color,
        };

        renderer.render_background((theme.value.corner_radius.get()).into(), background_color);

        renderer.render_text(
            &self.cached_values,
            theme.value.text_offset.get(),
            self.cached_color.invert(),
            theme.value.font_size.get(),
        );
    }
}
