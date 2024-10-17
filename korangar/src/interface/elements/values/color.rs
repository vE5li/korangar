use korangar_interface::elements::{Element, ElementState};
use korangar_interface::event::{ChangeEvent, ClickAction, HoverInformation};
use korangar_interface::layout::PlacementResolver;

use crate::graphics::Color;
use crate::input::MouseInputMode;
use crate::interface::application::InterfaceSettings;
use crate::interface::layout::{ScreenClip, ScreenPosition};
use crate::interface::theme::InterfaceTheme;
use crate::interface::windows::ColorWindow;
use crate::renderer::InterfaceRenderer;

pub struct MutableColorValue {
    name: String,
    reference: &'static Color,
    change_event: Option<ChangeEvent>,
    cached_color: Color,
    cached_values: String,
    state: ElementState<InterfaceSettings>,
}

impl MutableColorValue {
    pub fn new(name: String, reference: &'static Color, change_event: Option<ChangeEvent>) -> Self {
        let cached_color = *reference;
        let cached_values = format!(
            "{}, {}, {}, {}",
            cached_color.red_as_u8(),
            cached_color.green_as_u8(),
            cached_color.blue_as_u8(),
            cached_color.alpha_as_u8()
        );
        let state = ElementState::default();

        Self {
            name,
            reference,
            change_event,
            cached_color,
            cached_values,
            state,
        }
    }
}

impl Element<InterfaceSettings> for MutableColorValue {
    fn get_state(&self) -> &ElementState<InterfaceSettings> {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState<InterfaceSettings> {
        &mut self.state
    }

    fn resolve(
        &mut self,
        placement_resolver: &mut PlacementResolver<InterfaceSettings>,
        _application: &InterfaceSettings,
        theme: &InterfaceTheme,
    ) {
        self.state.resolve(placement_resolver, &theme.value.size_bound);
    }

    fn update(&mut self) -> Option<ChangeEvent> {
        let current_color = *self.reference;

        if self.cached_color != current_color {
            self.cached_color = current_color;
            self.cached_values = format!(
                "{}, {}, {}, {}",
                self.cached_color.red_as_u8(),
                self.cached_color.green_as_u8(),
                self.cached_color.blue_as_u8(),
                self.cached_color.alpha_as_u8()
            );
            return Some(ChangeEvent::RENDER_WINDOW);
        }

        None
    }

    fn hovered_element(&self, mouse_position: ScreenPosition, mouse_mode: &MouseInputMode) -> HoverInformation<InterfaceSettings> {
        match mouse_mode {
            MouseInputMode::None => self.state.hovered_element(mouse_position),
            _ => HoverInformation::Missed,
        }
    }

    fn left_click(&mut self, _force_update: &mut bool) -> Vec<ClickAction<InterfaceSettings>> {
        vec![ClickAction::OpenWindow(Box::new(ColorWindow::new(
            self.name.clone(),
            self.reference,
            self.change_event,
        )))]
    }

    fn render(
        &self,
        renderer: &InterfaceRenderer,
        application: &InterfaceSettings,
        theme: &InterfaceTheme,
        parent_position: ScreenPosition,
        screen_clip: ScreenClip,
        hovered_element: Option<&dyn Element<InterfaceSettings>>,
        _focused_element: Option<&dyn Element<InterfaceSettings>>,
        _mouse_mode: &MouseInputMode,
        _second_theme: bool,
    ) {
        let mut renderer = self.state.element_renderer(renderer, application, parent_position, screen_clip);

        let background_color = match self.is_element_self(hovered_element) {
            true => self.cached_color.shade(),
            false => self.cached_color,
        };

        renderer.render_background(theme.value.corner_radius.get(), background_color);

        renderer.render_text(
            &self.cached_values,
            theme.value.text_offset.get(),
            self.cached_color.invert(),
            theme.value.font_size.get(),
        );
    }
}
