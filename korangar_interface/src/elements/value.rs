use rust_state::Tracker;

use crate::application::{Application, InterfaceRenderer};
use crate::elements::{Element, ElementState};
use crate::layout::PlacementResolver;
use crate::theme::ValueTheme;

pub struct StringValue<App>
where
    App: Application,
{
    value: String,
    state: ElementState<App>,
}

impl<App> StringValue<App>
where
    App: Application,
{
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            state: Default::default(),
        }
    }
}

impl<App> Element<App> for StringValue<App>
where
    App: Application,
{
    fn get_state(&self) -> &ElementState<App> {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState<App> {
        &mut self.state
    }

    fn resolve(&mut self, state: &Tracker<App>, theme_selector: App::ThemeSelector, placement_resolver: &mut PlacementResolver<App>) {
        let size_bound = state.get_safe(&ValueTheme::size_bound(theme_selector));

        self.state.resolve(placement_resolver, &size_bound);
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

        renderer.render_background(
            *state.get_safe(&ValueTheme::corner_radius(theme_selector)),
            *state.get_safe(&ValueTheme::hovered_background_color(theme_selector)),
        );

        renderer.render_text(
            &self.value,
            *state.get_safe(&ValueTheme::text_offset(theme_selector)),
            *state.get_safe(&ValueTheme::foreground_color(theme_selector)),
            *state.get_safe(&ValueTheme::font_size(theme_selector)),
        );
    }
}
