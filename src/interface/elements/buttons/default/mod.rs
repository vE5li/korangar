mod builder;

pub use self::builder::ButtonBuilder;
use crate::graphics::{InterfaceRenderer, Renderer};
use crate::input::MouseInputMode;
use crate::interface::*;

pub struct Button<TEXT, EVENT>
where
    TEXT: AsRef<str> + 'static,
    EVENT: ElementEvent + 'static,
{
    text: TEXT,
    event: EVENT,
    disabled_selector: Option<Selector>,
    foreground_color: Option<ColorSelector>,
    background_color: Option<ColorSelector>,
    width_bound: DimensionBound,
    state: ElementState,
}

impl<TEXT, EVENT> Button<TEXT, EVENT>
where
    TEXT: AsRef<str> + 'static,
    EVENT: ElementEvent + 'static,
{
    fn is_disabled(&self) -> bool {
        self.disabled_selector.as_ref().map(|selector| !selector()).unwrap_or(false)
    }
}

impl<TEXT: AsRef<str> + 'static, EVENT: ElementEvent> Element for Button<TEXT, EVENT> {
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
        let size_bound = self.width_bound.add_height(theme.button.height_bound);
        self.state.resolve(placement_resolver, &size_bound);
    }

    fn hovered_element(&self, mouse_position: ScreenPosition, mouse_mode: &MouseInputMode) -> HoverInformation {
        match mouse_mode {
            MouseInputMode::None => self.state.hovered_element(mouse_position),
            _ => HoverInformation::Missed,
        }
    }

    fn left_click(&mut self, _force_update: &mut bool) -> Vec<ClickAction> {
        match self.is_disabled() {
            true => Vec::new(),
            false => self.event.trigger(),
        }
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

        let foreground_color = if disabled {
            theme.button.disabled_foreground_color.get()
        } else {
            self.foreground_color
                .as_ref()
                .map(|closure| closure(theme))
                .unwrap_or(theme.button.foreground_color.get())
        };

        renderer.render_text(
            self.text.as_ref(),
            theme.button.text_offset.get(),
            foreground_color,
            theme.button.font_size.get(),
        );
    }
}
