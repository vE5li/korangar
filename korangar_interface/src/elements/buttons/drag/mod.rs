mod builder;

pub use self::builder::DragButtonBuilder;
use crate::application::{Application, InterfaceRenderer, MouseInputModeTrait};
use crate::elements::{Element, ElementState};
use crate::event::{ClickAction, HoverInformation};
use crate::layout::{DimensionBound, PlacementResolver};
use crate::theme::{InterfaceTheme, WindowTheme};

pub struct DragButton<App>
where
    App: Application,
{
    title: String,
    width_bound: DimensionBound,
    state: ElementState<App>,
}

impl<App> Element<App> for DragButton<App>
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
        let size_bound = self.width_bound.add_height(theme.window().title_height());

        self.state.resolve(placement_resolver, &size_bound);
    }

    fn is_focusable(&self) -> bool {
        false
    }

    fn hovered_element(&self, mouse_position: App::Position, mouse_mode: &App::MouseInputMode) -> HoverInformation<App> {
        if mouse_mode.is_none() {
            self.state.hovered_element(mouse_position)
        } else if mouse_mode.is_self_dragged(self) {
            HoverInformation::Hovered
        } else {
            HoverInformation::Missed
        }
    }

    fn left_click(&mut self, _force_update: &mut bool) -> Vec<ClickAction<App>> {
        vec![ClickAction::MoveInterface]
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
        _focused_element: Option<&dyn Element<App>>,
        _mouse_mode: &App::MouseInputMode,
        _second_theme: bool,
    ) {
        let mut renderer = self
            .state
            .element_renderer(render_target, renderer, application, parent_position, screen_clip);

        if self.is_element_self(hovered_element) {
            renderer.render_background(theme.window().title_corner_radius(), theme.window().title_background_color());
        }

        renderer.render_text(
            &self.title,
            theme.window().text_offset(),
            theme.window().foreground_color(),
            theme.window().font_size(),
        );
    }
}
