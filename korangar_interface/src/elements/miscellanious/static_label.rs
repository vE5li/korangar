use rust_state::Tracker;

use crate::application::{Application, InterfaceRenderer, PartialSizeTrait, ScalingTrait, SizeTrait};
use crate::elements::{Element, ElementState};
use crate::layout::{Dimension, PlacementResolver};
use crate::theme::LabelTheme;

pub struct StaticLabel<App>
where
    App: Application,
{
    label: String,
    state: ElementState<App>,
}

impl<App> StaticLabel<App>
where
    App: Application,
{
    pub fn new(label: String) -> Self {
        Self {
            label,
            state: Default::default(),
        }
    }
}

impl<App> Element<App> for StaticLabel<App>
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
        let mut size_bound = state.get_safe(&LabelTheme::size_bound(theme_selector)).clone();

        let size = placement_resolver.get_text_dimensions(
            &self.label,
            *state.get_safe(&LabelTheme::font_size(theme_selector)),
            *state.get_safe(&LabelTheme::text_offset(theme_selector)),
            *state.get_safe(&App::ScaleSelector::default()),
            placement_resolver.get_available().width() / 2.0, // TODO: make better
        );

        size_bound.height = Dimension::Absolute(f32::max(
            size.height() / state.get_safe(&App::ScaleSelector::default()).get_factor(),
            14.0,
        )); // TODO: make better

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
            *state.get_safe(&LabelTheme::corner_radius(theme_selector)),
            *state.get_safe(&LabelTheme::background_color(theme_selector)),
        );

        renderer.render_text(
            &self.label,
            *state.get_safe(&LabelTheme::text_offset(theme_selector)),
            *state.get_safe(&LabelTheme::foreground_color(theme_selector)),
            *state.get_safe(&LabelTheme::font_size(theme_selector)),
        );
    }
}
