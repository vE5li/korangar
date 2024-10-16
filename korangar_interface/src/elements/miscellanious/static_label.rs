use crate::application::{Application, PartialSizeTrait, ScalingTrait, SizeTrait};
use crate::elements::{Element, ElementState};
use crate::layout::{Dimension, PlacementResolver};
use crate::theme::{InterfaceTheme, LabelTheme};

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

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver<App>, application: &App, theme: &App::Theme) {
        let mut size_bound = theme.label().size_bound();

        let size = placement_resolver.get_text_dimensions(
            &self.label,
            theme.label().font_size(),
            theme.label().text_offset(),
            application.get_scaling(),
            placement_resolver.get_available().width() / 2.0, // TODO: make better
        );

        size_bound.height = Dimension::Absolute(f32::max(size.height() / application.get_scaling().get_factor(), 14.0)); // TODO: make better

        self.state.resolve(placement_resolver, &size_bound);
    }

    fn render(
        &self,
        renderer: &App::Renderer,
        application: &App,
        theme: &App::Theme,
        parent_position: App::Position,
        screen_clip: App::Clip,
        _hovered_element: Option<&dyn Element<App>>,
        _focused_element: Option<&dyn Element<App>>,
        _mouse_mode: &App::MouseInputMode,
        _second_theme: bool,
    ) {
        let mut renderer = self.state.element_renderer(renderer, application, parent_position, screen_clip);

        renderer.render_background(theme.label().corner_radius(), theme.label().background_color());

        renderer.render_text(
            &self.label,
            theme.label().text_offset(),
            theme.label().foreground_color(),
            theme.label().font_size(),
        );
    }
}
