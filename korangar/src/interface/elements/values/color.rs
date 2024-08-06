use korangar_interface::elements::{Element, ElementState};
use korangar_interface::event::{ChangeEvent, ClickAction, HoverInformation};
use korangar_interface::layout::PlacementResolver;
use korangar_interface::theme::ValueTheme;
use rust_state::{Context, View};

use crate::graphics::{Color, InterfaceRenderer, Renderer};
use crate::input::MouseInputMode;
use crate::interface::application::ThemeSelector2;
use crate::interface::layout::{ScreenClip, ScreenPosition};
use crate::interface::theme::InterfaceTheme;
use crate::interface::windows::ColorWindow;
use crate::{GameState, GameStateHoveredElementPath};

pub struct MutableColorValue {
    name: String,
    reference: &'static Color,
    change_event: Option<ChangeEvent>,
    cached_color: Color,
    cached_values: String,
    state: ElementState<GameState>,
}

impl MutableColorValue {
    pub fn new(name: String, reference: &'static Color, change_event: Option<ChangeEvent>) -> Self {
        let cached_color = *reference;
        let cached_values = format!(
            "{}, {}, {}, {}",
            cached_color.red_as_u8(),
            cached_color.green_as_u8(),
            cached_color.blue_as_u8(),
            cached_color.alpha_as_u8()
        );
        let state = ElementState::default();

        Self {
            name,
            reference,
            change_event,
            cached_color,
            cached_values,
            state,
        }
    }
}

impl Element<GameState> for MutableColorValue {
    fn get_state(&self) -> &ElementState<GameState> {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState<GameState> {
        &mut self.state
    }

    fn resolve(&mut self, state: &View<GameState>, theme_selector: ThemeSelector2, placement_resolver: &mut PlacementResolver<GameState>) {
        let size_bound = state.get_safe(&ValueTheme::size_bound(theme_selector));
        self.state.resolve(placement_resolver, size_bound);
    }

    fn update(&mut self) -> Option<ChangeEvent> {
        let current_color = *self.reference;

        if self.cached_color != current_color {
            self.cached_color = current_color;
            self.cached_values = format!(
                "{}, {}, {}, {}",
                self.cached_color.red_as_u8(),
                self.cached_color.green_as_u8(),
                self.cached_color.blue_as_u8(),
                self.cached_color.alpha_as_u8()
            );
            return Some(ChangeEvent::RENDER_WINDOW);
        }

        None
    }

    fn hovered_element(&self, mouse_position: ScreenPosition, mouse_mode: &MouseInputMode) -> HoverInformation<GameState> {
        match mouse_mode {
            MouseInputMode::None => self.state.hovered_element(mouse_position),
            _ => HoverInformation::Missed,
        }
    }

    fn left_click(&mut self, _state: &Context<GameState>, _force_update: &mut bool) -> Vec<ClickAction<GameState>> {
        vec![ClickAction::OpenWindow(Box::new(ColorWindow::new(
            self.name.clone(),
            self.reference,
            self.change_event,
        )))]
    }

    fn render(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        renderer: &InterfaceRenderer,
        state: &View<GameState>,
        theme_selector: ThemeSelector2,
        parent_position: ScreenPosition,
        screen_clip: ScreenClip,
        _second_theme: bool,
    ) {
        let mut renderer = self
            .state
            .element_renderer(render_target, renderer, state, parent_position, screen_clip);

        let hovered_element = state.get_safe(&GameStateHoveredElementPath::default());
        let background_color = match self.is_cell_self(&hovered_element) {
            true => self.cached_color.shade(),
            false => self.cached_color,
        };

        renderer.render_background(*state.get_safe(&ValueTheme::corner_radius(theme_selector)), background_color);

        renderer.render_text(
            &self.cached_values,
            *state.get_safe(&ValueTheme::text_offset(theme_selector)),
            self.cached_color.invert(),
            *state.get_safe(&ValueTheme::font_size(theme_selector)),
        );
    }
}
