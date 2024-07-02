mod builder;

use rust_state::{Selector, Tracker};

pub use self::builder::StateButtonBuilder;
use crate::application::{Application, InterfaceRenderer, MouseInputModeTrait};
use crate::elements::{Element, ElementState};
use crate::event::{ClickAction, HoverInformation};
use crate::layout::{DimensionBound, PlacementResolver};
use crate::theme::ButtonTheme;
use crate::ElementEvent;

pub struct StateButton<App, Text, Event, State>
where
    App: Application,
    Text: AsRef<str> + 'static,
    Event: ElementEvent<App> + 'static,
    State: for<'a> Selector<'a, App, bool>,
{
    text: Text,
    event: Event,
    remote: State,
    width_bound: DimensionBound,
    transparent_background: bool,
    state: ElementState<App>,
}

impl<App, Text, Event, State> Element<App> for StateButton<App, Text, Event, State>
where
    App: Application,
    Text: AsRef<str> + 'static,
    Event: ElementEvent<App> + 'static,
    State: for<'a> Selector<'a, App, bool>,
{
    fn get_state(&self) -> &ElementState<App> {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState<App> {
        &mut self.state
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

    fn left_click(&mut self, _force_update: &mut bool) -> Vec<ClickAction<App>> {
        self.event.trigger()
    }

    fn render(
        &self,
        render_target: &mut <App::Renderer as InterfaceRenderer<App>>::Target,
        renderer: &App::Renderer,
        application: &Tracker<App>,
        theme_selector: App::ThemeSelector,
        parent_position: App::Position,
        screen_clip: App::Clip,
        _second_theme: bool,
    ) {
        let mut renderer = self
            .state
            .element_renderer(render_target, renderer, application, parent_position, screen_clip);

        let hovered_element = application.get_safe(&App::HoveredElementSelector::default());
        let focused_element = application.get_safe(&App::FocusedElementSelector::default());
        let highlighted = self.is_element_self(hovered_element) || self.is_element_self(focused_element);

        if !self.transparent_background {
            let background_color = match highlighted {
                true => application.get_safe(&ButtonTheme::hovered_background_color(theme_selector)),
                false => application.get_safe(&ButtonTheme::background_color(theme_selector)),
            };

            renderer.render_background(
                *application.get_safe(&ButtonTheme::corner_radius(theme_selector)),
                *background_color,
            );
        }

        let foreground_color = match self.transparent_background && highlighted {
            true => application.get_safe(&ButtonTheme::hovered_foreground_color(theme_selector)),
            false => application.get_safe(&ButtonTheme::foreground_color(theme_selector)),
        };

        renderer.render_checkbox(
            *application.get_safe(&ButtonTheme::icon_offset(theme_selector)),
            *application.get_safe(&ButtonTheme::icon_size(theme_selector)),
            *foreground_color,
            application.get(&self.remote).cloned().unwrap_or_default(),
        );

        renderer.render_text(
            self.text.as_ref(),
            *application.get_safe(&ButtonTheme::icon_text_offset(theme_selector)),
            *foreground_color,
            *application.get_safe(&ButtonTheme::font_size(theme_selector)),
        );
    }
}
