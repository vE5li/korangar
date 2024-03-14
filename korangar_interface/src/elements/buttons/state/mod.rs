mod builder;

pub use self::builder::StateButtonBuilder;
use crate::application::{Application, InterfaceRenderer, MouseInputModeTrait};
use crate::elements::{Element, ElementState};
use crate::event::{ChangeEvent, ClickAction, HoverInformation};
use crate::layout::{DimensionBound, PlacementResolver};
use crate::state::{Remote, RemoteClone};
use crate::theme::{ButtonTheme, InterfaceTheme};
use crate::ElementEvent;

// FIX: State button won't redraw just because the state changes
pub struct StateButton<App, Text, Event, State>
where
    App: Application,
    Text: AsRef<str> + 'static,
    Event: ElementEvent<App> + 'static,
    State: Remote<bool>,
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
    State: Remote<bool>,
{
    fn get_state(&self) -> &ElementState<App> {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState<App> {
        &mut self.state
    }

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver<App>, _application: &App, theme: &App::Theme) {
        let size_bound = self.width_bound.add_height(theme.button().height_bound());
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

    fn update(&mut self) -> Option<ChangeEvent> {
        self.remote.consume_changed().then_some(ChangeEvent::RENDER_WINDOW)
    }

    fn render(
        &self,
        render_target: &mut <App::Renderer as InterfaceRenderer<App>>::Target,
        renderer: &App::Renderer,
        application: &App,
        theme: &App::Theme,
        parent_position: App::Position,
        screen_clip: App::Clip,
        hovered_element: Option<&dyn Element<App>>,
        focused_element: Option<&dyn Element<App>>,
        _mouse_mode: &App::MouseInputMode,
        _second_theme: bool,
    ) {
        let mut renderer = self
            .state
            .element_renderer(render_target, renderer, application, parent_position, screen_clip);

        let highlighted = self.is_element_self(hovered_element) || self.is_element_self(focused_element);

        if !self.transparent_background {
            let background_color = match highlighted {
                true => theme.button().hovered_background_color(),
                false => theme.button().background_color(),
            };

            renderer.render_background(theme.button().corner_radius(), background_color);
        }

        let foreground_color = match self.transparent_background && highlighted {
            true => theme.button().hovered_foreground_color(),
            false => theme.button().foreground_color(),
        };

        renderer.render_checkbox(
            theme.button().icon_offset(),
            theme.button().icon_size(),
            foreground_color.clone(),
            self.remote.cloned(),
        );

        renderer.render_text(
            self.text.as_ref(),
            theme.button().icon_text_offset(),
            foreground_color,
            theme.button().font_size(),
        );
    }
}
