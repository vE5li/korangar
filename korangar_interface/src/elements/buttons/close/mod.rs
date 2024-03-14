mod builder;

pub use self::builder::CloseButtonBuilder;
use crate::application::{Application, InterfaceRenderer, MouseInputModeTrait, PartialSizeTraitExt};
use crate::elements::{Element, ElementState};
use crate::event::{ClickAction, HoverInformation};
use crate::layout::PlacementResolver;
use crate::theme::{CloseButtonTheme, InterfaceTheme};

pub struct CloseButton<App>
where
    App: Application,
{
    state: ElementState<App>,
}

impl<App> Element<App> for CloseButton<App>
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
        let (size, position) = placement_resolver.allocate_right(&theme.close_button().size_bound());
        self.state.cached_size = size.finalize();
        self.state.cached_position = position;
    }

    fn is_focusable(&self) -> bool {
        false
    }

    fn hovered_element(&self, mouse_position: App::Position, mouse_mode: &App::MouseInputMode) -> HoverInformation<App> {
        match mouse_mode.is_none() {
            true => self.state.hovered_element(mouse_position),
            false => HoverInformation::Missed,
        }
    }

    fn left_click(&mut self, _force_update: &mut bool) -> Vec<ClickAction<App>> {
        vec![ClickAction::CloseWindow]
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

        let background_color = match self.is_element_self(hovered_element) || self.is_element_self(focused_element) {
            true => theme.close_button().hovered_background_color(),
            false => theme.close_button().background_color(),
        };

        renderer.render_background(theme.close_button().corner_radius(), background_color);

        renderer.render_text(
            "X",
            theme.close_button().text_offset(),
            theme.close_button().foreground_color(),
            theme.close_button().font_size(),
        );
    }
}
