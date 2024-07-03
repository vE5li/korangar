mod builder;

use rust_state::{Context, Tracker};

pub use self::builder::DragButtonBuilder;
use crate::application::{Application, InterfaceRenderer, MouseInputModeTrait};
use crate::elements::{Element, ElementState};
use crate::event::{ClickAction, HoverInformation};
use crate::layout::{DimensionBound, PlacementResolver};
use crate::theme::WindowTheme;

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

    fn resolve(&mut self, application: &Tracker<App>, theme_selector: App::ThemeSelector, placement_resolver: &mut PlacementResolver<App>) {
        let title_height = *application.get_safe(&WindowTheme::title_height(theme_selector));
        let size_bound = self.width_bound.add_height(title_height);

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

    fn left_click(&mut self, _state: &Context<App>, _force_update: &mut bool) -> Vec<ClickAction<App>> {
        vec![ClickAction::MoveInterface]
    }

    fn render(
        &self,
        render_target: &mut <App::Renderer as InterfaceRenderer<App>>::Target,
        renderer: &App::Renderer,
        application: &Tracker<App>,
        theme_selector: App::ThemeSelector,
        parent_position: App::Position,
        screen_clip: App::Clip,
        _second_theme: bool,
    ) {
        let mut renderer = self
            .state
            .element_renderer(render_target, renderer, application, parent_position, screen_clip);

        let hovered_element = application.get_safe(&App::HoveredElementSelector::default());

        if self.is_element_self(hovered_element) {
            let title_corner_radius = *application.get_safe(&WindowTheme::title_corner_radius(theme_selector));
            let title_background_color = *application.get_safe(&WindowTheme::title_background_color(theme_selector));

            renderer.render_background(title_corner_radius, title_background_color);
        }

        let text_offset = *application.get_safe(&WindowTheme::text_offset(theme_selector));
        let foreground_color = *application.get_safe(&WindowTheme::foreground_color(theme_selector));
        let font_size = *application.get_safe(&WindowTheme::font_size(theme_selector));

        renderer.render_text(&self.title, text_offset, foreground_color, font_size);
    }
}
