use crate::application::Application;
use crate::elements::{Element, ElementState};
use crate::layout::PlacementResolver;
use crate::theme::{InterfaceTheme, ValueTheme};

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

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver<App>, _application: &App, theme: &App::Theme) {
        self.state.resolve(placement_resolver, &theme.value().size_bound());
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

        renderer.render_background(theme.value().corner_radius(), theme.value().hovered_background_color());

        renderer.render_text(
            &self.value,
            theme.value().text_offset(),
            theme.value().foreground_color(),
            theme.value().font_size(),
        );
    }
}
