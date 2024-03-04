mod builder;

use procedural::dimension_bound;

pub use self::builder::StateButtonBuilder;
use crate::graphics::{InterfaceRenderer, Renderer};
use crate::input::MouseInputMode;
use crate::interface::{Element, *};

type StateSelector = Box<dyn Fn(&StateProvider) -> bool + 'static>;

// FIX: State button won't redraw just because the state changes
pub struct StateButton<TEXT, EVENT>
where
    TEXT: AsRef<str> + 'static,
    EVENT: ElementEvent + 'static,
{
    text: TEXT,
    event: EVENT,
    selector: StateSelector,
    width_bound: Option<DimensionBound>,
    transparent_background: bool,
    state: ElementState,
}

impl<TEXT, EVENT> Element for StateButton<TEXT, EVENT>
where
    TEXT: AsRef<str> + 'static,
    EVENT: ElementEvent + 'static,
{
    fn get_state(&self) -> &ElementState {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState {
        &mut self.state
    }

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, _interface_settings: &InterfaceSettings, theme: &InterfaceTheme) {
        let size_bound = self
            .width_bound
            .as_ref()
            .unwrap_or(&dimension_bound!(100%))
            .add_height(theme.button.height_bound);

        self.state.resolve(placement_resolver, &size_bound);
    }

    fn hovered_element(&self, mouse_position: ScreenPosition, mouse_mode: &MouseInputMode) -> HoverInformation {
        match mouse_mode {
            MouseInputMode::None => self.state.hovered_element(mouse_position),
            _ => HoverInformation::Missed,
        }
    }

    fn left_click(&mut self, _force_update: &mut bool) -> Vec<ClickAction> {
        self.event.trigger()
    }

    fn render(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        renderer: &InterfaceRenderer,
        state_provider: &StateProvider,
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

        let highlighted = self.is_element_self(hovered_element) || self.is_element_self(focused_element);

        if !self.transparent_background {
            let background_color = match highlighted {
                true => theme.button.hovered_background_color.get(),
                false => theme.button.background_color.get(),
            };

            renderer.render_background(theme.button.corner_radius.get(), background_color);
        }

        let foreground_color = match self.transparent_background && highlighted {
            true => theme.button.hovered_foreground_color.get(),
            false => theme.button.foreground_color.get(),
        };

        renderer.render_checkbox(
            theme.button.icon_offset.get(),
            theme.button.icon_size.get(),
            foreground_color,
            (self.selector)(state_provider),
        );

        renderer.render_text(
            self.text.as_ref(),
            theme.button.icon_text_offset.get(),
            foreground_color,
            theme.button.font_size.get(),
        );
    }
}
