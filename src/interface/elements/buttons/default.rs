use procedural::dimension;

use crate::graphics::{InterfaceRenderer, Renderer};
use crate::input::MouseInputMode;
use crate::interface::*;

pub struct Button<T, E>
where
    T: AsRef<str> + 'static,
    E: ElementEvent + 'static,
{
    text: Option<T>,
    event: Option<E>,
    disabled_selector: Option<Selector>,
    foreground_color: Option<ColorSelector>,
    background_color: Option<ColorSelector>,
    width_constraint: Option<DimensionConstraint>,
    state: ElementState,
}

// HACK: Workaround for Rust incorrect trait bounds when deriving Option<T>
// where T: !Default.
impl<T, E> Default for Button<T, E>
where
    T: AsRef<str> + 'static,
    E: ElementEvent + 'static,
{
    fn default() -> Self {
        Self {
            text: Default::default(),
            event: Default::default(),
            disabled_selector: Default::default(),
            foreground_color: Default::default(),
            background_color: Default::default(),
            width_constraint: Default::default(),
            state: Default::default(),
        }
    }
}

impl<T, E> Button<T, E>
where
    T: AsRef<str> + 'static,
    E: ElementEvent + 'static,
{
    pub fn with_text(mut self, text: T) -> Self {
        self.text = Some(text);
        self
    }

    pub fn with_event(mut self, event: E) -> Self {
        self.event = Some(event);
        self
    }

    pub fn with_disabled_selector(mut self, disabled_selector: impl Fn() -> bool + 'static) -> Self {
        self.disabled_selector = Some(Box::new(disabled_selector));
        self
    }

    pub fn with_foreground_color(mut self, foreground_color: impl Fn(&InterfaceTheme) -> Color + 'static) -> Self {
        self.foreground_color = Some(Box::new(foreground_color));
        self
    }

    pub fn with_background_color(mut self, background_color: impl Fn(&InterfaceTheme) -> Color + 'static) -> Self {
        self.background_color = Some(Box::new(background_color));
        self
    }

    pub fn with_width(mut self, width_constraint: DimensionConstraint) -> Self {
        self.width_constraint = Some(width_constraint);
        self
    }

    fn is_disabled(&self) -> bool {
        self.disabled_selector.as_ref().map(|selector| !selector()).unwrap_or(false)
    }
}

impl<T: AsRef<str> + 'static, E: ElementEvent> Element for Button<T, E> {
    fn get_state(&self) -> &ElementState {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState {
        &mut self.state
    }

    fn is_focusable(&self) -> bool {
        !self.is_disabled()
    }

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, _interface_settings: &InterfaceSettings, theme: &InterfaceTheme) {
        let size_constraint = self
            .width_constraint
            .as_ref()
            .unwrap_or(&dimension!(100%))
            .add_height(theme.button.height_constraint);

        self.state.resolve(placement_resolver, &size_constraint);
    }

    fn hovered_element(&self, mouse_position: ScreenPosition, mouse_mode: &MouseInputMode) -> HoverInformation {
        match mouse_mode {
            MouseInputMode::None => self.state.hovered_element(mouse_position),
            _ => HoverInformation::Missed,
        }
    }

    fn left_click(&mut self, _force_update: &mut bool) -> Vec<ClickAction> {
        if self.is_disabled() {
            return Vec::new();
        }

        self.event.as_mut().map(|event| event.trigger()).unwrap_or_default()
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

        let disabled = self.is_disabled();
        let background_color = match self.is_element_self(hovered_element) || self.is_element_self(focused_element) {
            _ if disabled => theme.button.disabled_background_color.get(),
            true => theme.button.hovered_background_color.get(),
            false if self.background_color.is_some() => (self.background_color.as_ref().unwrap())(theme),
            false => theme.button.background_color.get(),
        };

        renderer.render_background(theme.button.corner_radius.get(), background_color);

        if let Some(text) = &self.text {
            let foreground_color = if disabled {
                theme.button.disabled_foreground_color.get()
            } else {
                self.foreground_color
                    .as_ref()
                    .map(|closure| closure(theme))
                    .unwrap_or(theme.button.foreground_color.get())
            };

            renderer.render_text(
                text.as_ref(),
                theme.button.text_offset.get(),
                foreground_color,
                theme.button.font_size.get(),
            );
        }
    }
}
