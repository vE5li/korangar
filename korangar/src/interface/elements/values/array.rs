use std::cmp::PartialOrd;
use std::fmt::Display;

use korangar_interface::elements::{Element, ElementDisplay, ElementState};
use korangar_interface::event::{ChangeEvent, ClickAction, HoverInformation};
use korangar_interface::layout::PlacementResolver;
use num::traits::NumOps;
use num::{NumCast, Zero};

use crate::graphics::{InterfaceRenderer, Renderer};
use crate::input::MouseInputMode;
use crate::interface::application::InterfaceSettings;
use crate::interface::layout::{ArrayType, ScreenClip, ScreenPosition};
use crate::interface::theme::InterfaceTheme;
use crate::interface::windows::ArrayWindow;

pub struct MutableArrayValue<T>
where
    T: ArrayType + ElementDisplay + Copy + PartialEq + 'static,
    T::Element: Zero + NumOps + NumCast + Copy + PartialOrd + Display + 'static,
    [(); T::ELEMENT_COUNT]:,
{
    name: String,
    reference: &'static T,
    minimum_value: T,
    maximum_value: T,
    change_event: Option<ChangeEvent>,
    cached_inner: T,
    cached_values: String,
    state: ElementState<InterfaceSettings>,
}

impl<T> MutableArrayValue<T>
where
    T: ArrayType + ElementDisplay + Copy + PartialEq + 'static,
    T::Element: Zero + NumOps + NumCast + Copy + PartialOrd + Display + 'static,
    [(); T::ELEMENT_COUNT]:,
{
    pub fn new(name: String, reference: &'static T, minimum_value: T, maximum_value: T, change_event: Option<ChangeEvent>) -> Self {
        let cached_inner = *reference;
        let cached_values = cached_inner.display();
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

impl<T> Element<InterfaceSettings> for MutableArrayValue<T>
where
    T: ArrayType + ElementDisplay + Copy + PartialEq + 'static,
    T::Element: Zero + NumOps + NumCast + Copy + PartialOrd + Display + 'static,
    [(); T::ELEMENT_COUNT]:,
{
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
            self.cached_values = self.cached_inner.display();
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
        let prototype_window = ArrayWindow::new(
            self.name.clone(),
            self.reference,
            self.minimum_value,
            self.maximum_value,
            self.change_event,
        );

        vec![ClickAction::OpenWindow(Box::new(prototype_window))]
    }

    fn render(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
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
        let mut renderer = self
            .state
            .element_renderer(render_target, renderer, application, parent_position, screen_clip);

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
