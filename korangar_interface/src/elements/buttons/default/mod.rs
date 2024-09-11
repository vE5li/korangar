mod builder;

pub use self::builder::ButtonBuilder;
use crate::application::{Application, InterfaceRenderer, MouseInputModeTrait};
use crate::elements::{Element, ElementState};
use crate::event::{ClickAction, HoverInformation};
use crate::layout::{DimensionBound, PlacementResolver};
use crate::theme::{ButtonTheme, InterfaceTheme};
use crate::{ColorSelector, ElementEvent, Selector};

pub struct Button<App, Text, Event>
where
    App: Application,
    Text: AsRef<str> + 'static,
    Event: ElementEvent<App> + 'static,
{
    text: Text,
    event: Event,
    disabled_selector: Option<Selector>,
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
        self.disabled_selector.as_ref().map(|selector| !selector()).unwrap_or(false)
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
        match self.is_disabled() {
            true => Vec::new(),
            false => self.event.trigger(),
        }
    }

    fn render(
        &self,
        render_target: &mut <App::Renderer as InterfaceRenderer<App>>::Target,
        render_pass: &mut App::RenderPass<'_>,
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
            .element_renderer(render_target, render_pass, renderer, application, parent_position, screen_clip);

        let disabled = self.is_disabled();
        let background_color = match self.is_element_self(hovered_element) || self.is_element_self(focused_element) {
            _ if disabled => theme.button().disabled_background_color(),
            true => theme.button().hovered_background_color(),
            false if self.background_color.is_some() => (self.background_color.as_ref().unwrap())(theme),
            false => theme.button().background_color(),
        };

        renderer.render_background(theme.button().corner_radius(), background_color);

        let foreground_color = if disabled {
            theme.button().disabled_foreground_color()
        } else {
            self.foreground_color
                .as_ref()
                .map(|closure| closure(theme))
                .unwrap_or(theme.button().foreground_color())
        };

        renderer.render_text(
            self.text.as_ref(),
            theme.button().text_offset(),
            foreground_color,
            theme.button().font_size(),
        );
    }
}
