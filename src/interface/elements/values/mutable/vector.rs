use std::cmp::PartialOrd;
use std::fmt::Display;

use cgmath::Array;
use num::traits::NumOps;
use num::{NumCast, Zero};

use crate::graphics::{InterfaceRenderer, Renderer};
use crate::input::MouseInputMode;
use crate::interface::{Element, *};

pub struct MutableVectorValue<T>
where
    T: Array + ElementDisplay + Copy + PartialEq + 'static,
    T::Element: Zero + NumOps + NumCast + Copy + PartialOrd + Display + 'static,
{
    name: String,
    inner_pointer: *const T,
    minimum_value: T,
    maximum_value: T,
    change_event: Option<ChangeEvent>,
    cached_inner: T,
    cached_values: String,
    state: ElementState,
}

impl<T> MutableVectorValue<T>
where
    T: Array + ElementDisplay + Copy + PartialEq + 'static,
    T::Element: Zero + NumOps + NumCast + Copy + PartialOrd + Display + 'static,
{
    pub fn new(name: String, inner_pointer: *const T, minimum_value: T, maximum_value: T, change_event: Option<ChangeEvent>) -> Self {
        let cached_inner = unsafe { *inner_pointer };
        let cached_values = cached_inner.display();
        let state = ElementState::default();

        Self {
            name,
            inner_pointer,
            minimum_value,
            maximum_value,
            change_event,
            cached_inner,
            cached_values,
            state,
        }
    }
}

impl<T> Element for MutableVectorValue<T>
where
    T: Array + ElementDisplay + Copy + PartialEq + 'static,
    T::Element: Zero + NumOps + NumCast + Copy + PartialOrd + Display + 'static,
{
    fn get_state(&self) -> &ElementState {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState {
        &mut self.state
    }

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, _interface_settings: &InterfaceSettings, theme: &Theme) {
        self.state.resolve(placement_resolver, &theme.value.size_constraint);
    }

    fn update(&mut self) -> Option<ChangeEvent> {
        let current_value = unsafe { *self.inner_pointer };

        if self.cached_inner != current_value {
            self.cached_inner = current_value;
            self.cached_values = self.cached_inner.display();
            return Some(ChangeEvent::RERENDER_WINDOW);
        }

        None
    }

    fn hovered_element(&self, mouse_position: Position, mouse_mode: &MouseInputMode) -> HoverInformation {
        match mouse_mode {
            MouseInputMode::None => self.state.hovered_element(mouse_position),
            _ => HoverInformation::Missed,
        }
    }

    fn left_click(&mut self, _force_update: &mut bool) -> Option<ClickAction> {
        let prototype_window = VectorWindow::new(
            self.name.clone(),
            self.inner_pointer,
            self.minimum_value,
            self.maximum_value,
            self.change_event,
        );

        Some(ClickAction::OpenWindow(Box::new(prototype_window)))
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
        _mouse_mode: &MouseInputMode,
        _second_theme: bool,
    ) {
        let mut renderer = self
            .state
            .element_renderer(render_target, renderer, interface_settings, parent_position, clip_size);

        let background_color = match self.is_element_self(hovered_element) {
            true => *theme.value.hovered_background_color,
            false => *theme.value.background_color,
        };

        renderer.render_background(*theme.value.border_radius, background_color);

        renderer.render_text(
            &self.cached_values,
            *theme.value.text_offset,
            *theme.value.foreground_color,
            *theme.value.font_size,
        );
    }
}
