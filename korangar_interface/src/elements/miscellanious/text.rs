use crate::application::{Application, FontSizeTrait, InterfaceRenderer, PositionTraitExt};
use crate::elements::{Element, ElementState};
use crate::layout::{Dimension, DimensionBound, PlacementResolver};
use crate::theme::{ButtonTheme, InterfaceTheme};
use crate::{ColorSelector, FontSizeSelector};

pub struct Text<App, T>
where
    App: Application,
    // TODO: Would be nice to call this `Text` but it makes `#[derive(Default)]` fail.
    T: AsRef<str> + 'static,
{
    text: Option<T>,
    foreground_color: Option<ColorSelector<App>>,
    width_bound: Option<DimensionBound>,
    font_size: Option<FontSizeSelector<App>>,
    state: ElementState<App>,
}

impl<App, T> Default for Text<App, T>
where
    App: Application,
    T: AsRef<str> + 'static,
{
    fn default() -> Self {
        Self {
            text: Default::default(),
            foreground_color: Default::default(),
            width_bound: Default::default(),
            font_size: Default::default(),
            state: Default::default(),
        }
    }
}

impl<App, T> Text<App, T>
where
    App: Application,
    T: AsRef<str> + 'static,
{
    pub fn with_text(mut self, text: T) -> Self {
        self.text = Some(text);
        self
    }

    pub fn with_foreground_color(mut self, foreground_color: impl Fn(&App::Theme) -> App::Color + 'static) -> Self {
        self.foreground_color = Some(Box::new(foreground_color));
        self
    }

    pub fn with_font_size(mut self, font_size: impl Fn(&App::Theme) -> App::FontSize + 'static) -> Self {
        self.font_size = Some(Box::new(font_size));
        self
    }

    pub fn with_width(mut self, width_bound: DimensionBound) -> Self {
        self.width_bound = Some(width_bound);
        self
    }

    fn get_font_size(&self, theme: &App::Theme) -> App::FontSize {
        self.font_size
            .as_ref()
            .map(|closure| closure(theme))
            .unwrap_or(theme.button().font_size())
    }
}

impl<App, T> Element<App> for Text<App, T>
where
    App: Application,
    T: AsRef<str> + 'static,
{
    fn get_state(&self) -> &ElementState<App> {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState<App> {
        &mut self.state
    }

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver<App>, _application: &App, theme: &App::Theme) {
        let height_bound = DimensionBound {
            size: Dimension::Absolute(self.get_font_size(theme).get_value()),
            minimum_size: None,
            maximum_size: None,
        };

        let size_bound = self
            .width_bound
            .as_ref()
            .unwrap_or(&DimensionBound::RELATIVE_ONE_HUNDRED)
            .add_height(height_bound);

        self.state.resolve(placement_resolver, &size_bound);
    }

    fn is_focusable(&self) -> bool {
        false
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

        let foreground_color = self
            .foreground_color
            .as_ref()
            .map(|closure| closure(theme))
            .unwrap_or(theme.button().foreground_color());

        let text = self.text.as_ref().unwrap();
        renderer.render_text(
            text.as_ref(),
            App::Position::zero(),
            foreground_color,
            self.get_font_size(theme),
        );
    }
}
