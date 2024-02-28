use num::Zero;
use procedural::dimension;

use crate::graphics::{Color, InterfaceRenderer, Renderer};
use crate::interface::*;

#[derive(Default)]
pub struct Text<T: AsRef<str> + 'static> {
    text: Option<T>,
    foreground_color: Option<ColorSelector>,
    width_constraint: Option<DimensionConstraint>,
    font_size: Option<FontSizeSelector>,
    state: ElementState,
}

impl<T: AsRef<str> + 'static> Text<T> {
    pub fn with_text(mut self, text: T) -> Self {
        self.text = Some(text);
        self
    }

    pub fn with_foreground_color(mut self, foreground_color: impl Fn(&InterfaceTheme) -> Color + 'static) -> Self {
        self.foreground_color = Some(Box::new(foreground_color));
        self
    }

    pub fn with_font_size(mut self, font_size: impl Fn(&InterfaceTheme) -> f32 + 'static) -> Self {
        self.font_size = Some(Box::new(font_size));
        self
    }

    pub fn with_width(mut self, width_constraint: DimensionConstraint) -> Self {
        self.width_constraint = Some(width_constraint);
        self
    }

    fn get_font_size(&self, theme: &InterfaceTheme) -> f32 {
        self.font_size
            .as_ref()
            .map(|closure| closure(theme))
            .unwrap_or(*theme.button.font_size)
    }
}

impl<T: AsRef<str> + 'static> Element for Text<T> {
    fn get_state(&self) -> &ElementState {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState {
        &mut self.state
    }

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, _interface_settings: &InterfaceSettings, theme: &InterfaceTheme) {
        let height_constraint = DimensionConstraint {
            size: Dimension::Absolute(self.get_font_size(theme)),
            minimum_size: None,
            maximum_size: None,
        };

        let size_constraint = self
            .width_constraint
            .as_ref()
            .unwrap_or(&dimension!(100%))
            .add_height(height_constraint);

        self.state.resolve(placement_resolver, &size_constraint);
    }

    fn is_focusable(&self) -> bool {
        false
    }

    fn render(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        renderer: &InterfaceRenderer,
        _state_provider: &StateProvider,
        interface_settings: &InterfaceSettings,
        theme: &InterfaceTheme,
        parent_position: Position,
        clip_size: ClipSize,
        _hovered_element: Option<&dyn Element>,
        _focused_element: Option<&dyn Element>,
        _mouse_mode: &MouseInputMode,
        _second_theme: bool,
    ) {
        let mut renderer = self
            .state
            .element_renderer(render_target, renderer, interface_settings, parent_position, clip_size);

        let foreground_color = self
            .foreground_color
            .as_ref()
            .map(|closure| closure(theme))
            .unwrap_or(*theme.button.foreground_color);

        let text = self.text.as_ref().unwrap();
        renderer.render_text(text.as_ref(), Vector2::zero(), foreground_color, self.get_font_size(theme));
    }
}
