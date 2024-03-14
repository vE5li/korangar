use crate::application::{Application, InterfaceRenderer};
use crate::elements::{Element, ElementState};
use crate::layout::{PlacementResolver, SizeBound};
use crate::theme::{InterfaceTheme, LabelTheme};

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

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver<App>, _application: &App, _theme: &App::Theme) {
        self.state.resolve(placement_resolver, &self.size_bound);
    }

    fn render(
        &self,
        render_target: &mut <App::Renderer as InterfaceRenderer<App>>::Target,
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
        let mut renderer = self
            .state
            .element_renderer(render_target, renderer, application, parent_position, screen_clip);

        renderer.render_text(
            &self.display,
            theme.label().text_offset(),
            theme.label().foreground_color(),
            theme.label().font_size(),
        );
    }
}
