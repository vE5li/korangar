use std::cmp::PartialOrd;
use std::fmt::Display;

use korangar_interface::elements::{Element, ElementDisplay, ElementState};
use korangar_interface::event::{ChangeEvent, ClickAction, HoverInformation};
use korangar_interface::layout::PlacementResolver;
use korangar_interface::theme::ValueTheme;
use num::traits::NumOps;
use num::{NumCast, Zero};
use rust_state::{Context, Tracker};

use crate::graphics::{InterfaceRenderer, Renderer};
use crate::input::MouseInputMode;
use crate::interface::application::ThemeSelector2;
use crate::interface::layout::{ArrayType, ScreenClip, ScreenPosition};
use crate::interface::windows::ArrayWindow;
use crate::GameState;

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
    state: ElementState<GameState>,
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

impl<T> Element<GameState> for MutableArrayValue<T>
where
    T: ArrayType + ElementDisplay + Copy + PartialEq + 'static,
    T::Element: Zero + NumOps + NumCast + Copy + PartialOrd + Display + 'static,
    [(); T::ELEMENT_COUNT]:,
{
    fn get_state(&self) -> &ElementState<GameState> {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState<GameState> {
        &mut self.state
    }

    fn resolve(
        &mut self,
        state: &Tracker<GameState>,
        theme_selector: ThemeSelector2,
        placement_resolver: &mut PlacementResolver<GameState>,
    ) {
        let size_bound = state.get_safe(&ValueTheme::size_bound(theme_selector));
        self.state.resolve(placement_resolver, size_bound);
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

    fn hovered_element(&self, mouse_position: ScreenPosition, mouse_mode: &MouseInputMode) -> HoverInformation<GameState> {
        match mouse_mode {
            MouseInputMode::None => self.state.hovered_element(mouse_position),
            _ => HoverInformation::Missed,
        }
    }

    fn left_click(&mut self, _state: &Context<GameState>, _force_update: &mut bool) -> Vec<ClickAction<GameState>> {
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
        state: &Tracker<GameState>,
        theme_selector: ThemeSelector2,
        parent_position: ScreenPosition,
        screen_clip: ScreenClip,
        _second_theme: bool,
    ) {
        let mut renderer = self
            .state
            .element_renderer(render_target, renderer, state, parent_position, screen_clip);

        let background_color = match self.is_element_self(hovered_element) {
            true => state.get_safe(&ValueTheme::hovered_background_color(theme_selector)),
            false => state.get_safe(&ValueTheme::background_color(theme_selector)),
        };

        renderer.render_background(*state.get_safe(&ValueTheme::corner_radius(theme_selector)), *background_color);

        renderer.render_text(
            &self.cached_values,
            *state.get_safe(&ValueTheme::text_offset(theme_selector)),
            *state.get_safe(&ValueTheme::foreground_color(theme_selector)),
            *state.get_safe(&ValueTheme::font_size(theme_selector)),
        );
    }
}
