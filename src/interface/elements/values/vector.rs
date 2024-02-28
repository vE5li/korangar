use std::fmt::Display;

use cgmath::Array;

use crate::graphics::{InterfaceRenderer, Renderer};
use crate::interface::{Element, *};

pub struct VectorValue<T>
where
    T: Array,
    T::Element: Copy + Display,
{
    value: T,
    display: String,
    state: ElementState,
}

impl<T> VectorValue<T>
where
    T: Array,
    T::Element: Copy + Display,
{

    pub fn new(value: T) -> Self {

        let display = (0..<T as Array>::len())
            .into_iter()
            .map(|index| format!("{:.01}", value[index]))
            .intersperse(", ".to_string())
            .collect();

        let state = ElementState::default();

        Self { value, display, state }
    }
}

impl<T> Element for VectorValue<T>
where
    T: Array,
    T::Element: Copy + Display,
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

    fn render(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        renderer: &InterfaceRenderer,
        _state_provider: &StateProvider,
        interface_settings: &InterfaceSettings,
        theme: &Theme,
        parent_position: Position,
        screen_clip: ClipSize,
        _hovered_element: Option<&dyn Element>,
        _focused_element: Option<&dyn Element>,
        _second_theme: bool,
    ) {

        let mut renderer = self
            .state
            .element_renderer(render_target, renderer, interface_settings, parent_position, screen_clip);

        renderer.render_background(*theme.value.corner_radius, *theme.value.hovered_background_color);

        renderer.render_text(
            &self.display,
            *theme.value.text_offset,
            *theme.value.foreground_color,
            *theme.value.font_size,
        );
    }
}
