mod builder;

use rust_state::{Context, Tracker};

pub use self::builder::ButtonBuilder;
use crate::application::{Application, InterfaceRenderer, MouseInputModeTrait};
use crate::elements::{Element, ElementState};
use crate::event::{ClickAction, HoverInformation};
use crate::layout::{DimensionBound, PlacementResolver};
use crate::theme::ButtonTheme;
use crate::{BaseSelector, ColorSelector, ElementEvent};

pub struct Button<App, Text, Event>
where
    App: Application,
    Text: AsRef<str> + 'static,
    Event: ElementEvent<App> + 'static,
{
    text: Text,
    event: Event,
    disabled_selector: Option<BaseSelector<App>>,
    foreground_color: Option<ColorSelector<App>>,
    background_color: Option<ColorSelector<App>>,
    width_bound: DimensionBound,
    state: ElementState<App>,
}

impl<App, Text, Event> Button<App, Text, Event>
where
    App: Application,
    Text: AsRef<str> + 'static,
    Event: ElementEvent<App> + 'static,
{
    fn is_disabled(&self) -> bool {
        // self.disabled_selector.as_ref().map(|selector| !selector()).unwrap_or(false)
        false
    }
}

impl<App, Text, Event> Element<App> for Button<App, Text, Event>
where
    App: Application,
    Text: AsRef<str> + 'static,
    Event: ElementEvent<App>,
{
    fn get_state(&self) -> &ElementState<App> {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState<App> {
        &mut self.state
    }

    fn is_focusable(&self) -> bool {
        !self.is_disabled()
    }

    fn resolve(&mut self, application: &Tracker<App>, theme_selector: App::ThemeSelector, placement_resolver: &mut PlacementResolver<App>) {
        let height_bound = *application.get_safe(&ButtonTheme::height_bound(theme_selector));
        let size_bound = self.width_bound.add_height(height_bound);

        self.state.resolve(placement_resolver, &size_bound);
    }

    fn hovered_element(&self, mouse_position: App::Position, mouse_mode: &App::MouseInputMode) -> HoverInformation<App> {
        match mouse_mode.is_none() {
            true => self.state.hovered_element(mouse_position),
            false => HoverInformation::Missed,
        }
    }

    fn left_click(&mut self, state: &Context<App>, _force_update: &mut bool) -> Vec<ClickAction<App>> {
        match self.is_disabled() {
            true => Vec::new(),
            false => self.event.trigger(state),
        }
    }

    fn render(
        &self,
        render_target: &mut <App::Renderer as InterfaceRenderer<App>>::Target,
        renderer: &App::Renderer,
        state: &Tracker<App>,
        theme_selector: App::ThemeSelector,
        parent_position: App::Position,
        screen_clip: App::Clip,
        _second_theme: bool,
    ) {
        let mut renderer = self
            .state
            .element_renderer(render_target, renderer, state, parent_position, screen_clip);

        let disabled = self.is_disabled();
        let corner_radius = state.get_safe(&ButtonTheme::corner_radius(theme_selector));

        let background_color = if disabled {
            *state.get_safe(&ButtonTheme::disabled_background_color(theme_selector))
        } else {
            let hovered_element = state.get_safe(&App::HoveredElementSelector::default());
            let focused_element = state.get_safe(&App::FocusedElementSelector::default());
            let highlighted = self.is_cell_self(&hovered_element) || self.is_cell_self(&focused_element);

            match highlighted {
                true => *state.get_safe(&ButtonTheme::hovered_background_color(theme_selector)),
                false if self.background_color.is_some() => (self.background_color.as_ref().unwrap())(state, theme_selector),
                false => *state.get_safe(&ButtonTheme::background_color(theme_selector)),
            }
        };

        renderer.render_background(*corner_radius, background_color);

        let foreground_color = if disabled {
            *state.get_safe(&ButtonTheme::disabled_foreground_color(theme_selector))
        } else {
            self.foreground_color
                .as_ref()
                .map(|closure| closure(state, theme_selector))
                .unwrap_or(*state.get_safe(&ButtonTheme::foreground_color(theme_selector)))
        };

        let text_offset = state.get_safe(&ButtonTheme::text_offset(theme_selector));
        let font_size = state.get_safe(&ButtonTheme::font_size(theme_selector));

        renderer.render_text(self.text.as_ref(), *text_offset, foreground_color, *font_size);
    }
}
