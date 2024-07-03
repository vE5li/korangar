mod builder;

use rust_state::{Context, Tracker};

pub use self::builder::CloseButtonBuilder;
use crate::application::{Application, InterfaceRenderer, MouseInputModeTrait, PartialSizeTraitExt};
use crate::elements::{Element, ElementState};
use crate::event::{ClickAction, HoverInformation};
use crate::layout::PlacementResolver;
use crate::theme::CloseButtonTheme;

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

    fn resolve(&mut self, application: &Tracker<App>, theme_selector: App::ThemeSelector, placement_resolver: &mut PlacementResolver<App>) {
        let size_bound = application.get_safe(&CloseButtonTheme::size_bound(theme_selector));
        let (size, position) = placement_resolver.allocate_right(size_bound);
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

    fn left_click(&mut self, _state: &Context<App>, _force_update: &mut bool) -> Vec<ClickAction<App>> {
        vec![ClickAction::CloseWindow]
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
        let focused_element = application.get_safe(&App::FocusedElementSelector::default());
        let highlighted = self.is_element_self(hovered_element) || self.is_element_self(focused_element);

        let corner_radius = *application.get_safe(&CloseButtonTheme::corner_radius(theme_selector));
        let background_color = match highlighted {
            true => application.get_safe(&CloseButtonTheme::hovered_background_color(theme_selector)),
            false => application.get_safe(&CloseButtonTheme::background_color(theme_selector)),
        };

        renderer.render_background(corner_radius, *background_color);

        let text_offset = *application.get_safe(&CloseButtonTheme::text_offset(theme_selector));
        let foreground_color = *application.get_safe(&CloseButtonTheme::foreground_color(theme_selector));
        let font_size = *application.get_safe(&CloseButtonTheme::font_size(theme_selector));

        renderer.render_text("X", text_offset, foreground_color, font_size);
    }
}
