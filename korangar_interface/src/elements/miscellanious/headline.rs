use rust_state::View;

use crate::application::{Application, InterfaceRenderer};
use crate::elements::{Element, ElementState};
use crate::layout::{PlacementResolver, SizeBound};
use crate::theme::LabelTheme;

pub struct Headline<App>
where
    App: Application,
{
    display: String,
    size_bound: SizeBound,
    state: ElementState<App>,
}

impl<App> Headline<App>
where
    App: Application,
{
    pub fn new(display: String, size_bound: SizeBound) -> Self {
        Self {
            display,
            size_bound,
            state: Default::default(),
        }
    }
}

impl<App> Element<App> for Headline<App>
where
    App: Application,
{
    fn get_state(&self) -> &ElementState<App> {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState<App> {
        &mut self.state
    }

    fn is_focusable(&self) -> bool {
        false
    }

    fn resolve(&mut self, _state: &View<App>, _theme_selector: App::ThemeSelector, placement_resolver: &mut PlacementResolver<App>) {
        self.state.resolve(placement_resolver, &self.size_bound);
    }

    fn render(
        &self,
        render_target: &mut <App::Renderer as InterfaceRenderer<App>>::Target,
        renderer: &App::Renderer,
        state: &View<App>,
        theme_selector: App::ThemeSelector,
        parent_position: App::Position,
        screen_clip: App::Clip,
        _second_theme: bool,
    ) {
        let mut renderer = self
            .state
            .element_renderer(render_target, renderer, state, parent_position, screen_clip);

        renderer.render_text(
            &self.display,
            *state.get_safe(&LabelTheme::text_offset(theme_selector)),
            *state.get_safe(&LabelTheme::foreground_color(theme_selector)),
            *state.get_safe(&LabelTheme::font_size(theme_selector)),
        );
    }
}
