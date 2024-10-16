use std::cmp::PartialOrd;
use std::fmt::Display;

use korangar_interface::elements::{Element, ElementState};
use korangar_interface::event::{ChangeEvent, ClickAction, HoverInformation};
use korangar_interface::layout::PlacementResolver;
use num::traits::NumOps;
use num::{NumCast, Zero};

use crate::input::MouseInputMode;
use crate::interface::application::InterfaceSettings;
use crate::interface::layout::{ScreenClip, ScreenPosition};
use crate::interface::theme::InterfaceTheme;
use crate::interface::windows::NumberWindow;
use crate::renderer::InterfaceRenderer;

pub struct MutableNumberValue<T: Zero + NumOps + NumCast + Copy + PartialOrd + Display + 'static> {
    name: String,
    reference: &'static T,
    minimum_value: T,
    maximum_value: T,
    change_event: Option<ChangeEvent>,
    cached_inner: T,
    cached_values: String,
    state: ElementState<InterfaceSettings>,
}

impl<T: Zero + NumOps + NumCast + Copy + PartialOrd + Display + 'static> MutableNumberValue<T> {
    pub fn new(name: String, reference: &'static T, minimum_value: T, maximum_value: T, change_event: Option<ChangeEvent>) -> Self {
        let cached_inner = *reference;
        let cached_values = format!("{cached_inner:.1}");
        let state = ElementState::default();

        Self {
            name,
            reference,
            minimum_value,
            maximum_value,
            change_event,
            cached_inner,
            cached_values,
            state,
        }
    }
}

impl<T: Zero + NumOps + NumCast + Copy + PartialOrd + Display + 'static> Element<InterfaceSettings> for MutableNumberValue<T> {
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
        let current_value = *self.reference;

        if self.cached_inner != current_value {
            self.cached_inner = current_value;
            self.cached_values = format!("{:.1}", self.cached_inner);
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
        vec![ClickAction::OpenWindow(Box::new(NumberWindow::new(
            self.name.clone(),
            self.reference,
            self.minimum_value,
            self.maximum_value,
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
            true => theme.value.hovered_background_color.get(),
            false => theme.value.background_color.get(),
        };

        renderer.render_background(theme.value.corner_radius.get(), background_color);

        renderer.render_text(
            &self.cached_values,
            theme.value.text_offset.get(),
            theme.value.foreground_color.get(),
            theme.value.font_size.get(),
        );
    }
}
