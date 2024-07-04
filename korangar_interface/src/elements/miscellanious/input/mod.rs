mod builder;

use std::fmt::Display;

use rust_state::{Context, SafeUnwrap, Selector, Tracker};

pub use self::builder::InputFieldBuilder;
use crate::application::{
    Application, CornerRadiusTraitExt, InterfaceRenderer, MouseInputModeTrait, PositionTrait, PositionTraitExt, ScalingTrait, SizeTrait,
};
use crate::elements::{Element, ElementState};
use crate::event::{ClickAction, HoverInformation};
use crate::layout::{DimensionBound, PlacementResolver};
use crate::theme::InputTheme;

/// Local type alias to simplify the builder.
type EnterAction<App> = Box<dyn FnMut(&Context<App>) -> Vec<ClickAction<App>>>;

pub struct InputField<Data, App, Text>
where
    Data: for<'a> Selector<'a, App, String> + SafeUnwrap,
    App: Application,
    Text: Display + 'static,
{
    input_state: Data,
    ghost_text: Text,
    enter_action: EnterAction<App>,
    length: usize,
    hidden: bool,
    width_bound: DimensionBound,
    state: ElementState<App>,
}

impl<Data, App, Text> InputField<Data, App, Text>
where
    Data: for<'a> Selector<'a, App, String> + SafeUnwrap,
    App: Application,
    Text: Display + 'static,
{
    fn remove_character(&self, state: &Context<App>) -> Vec<ClickAction<App>> {
        let mut current_input = state.get_safe(&self.input_state).clone();

        if !current_input.is_empty() {
            current_input.pop();
            state.update_value(&self.input_state, current_input);
        }

        return vec![];
    }

    fn add_character(&self, state: &Context<App>, character: char) -> Vec<ClickAction<App>> {
        let mut current_input = state.get_safe(&self.input_state).clone();

        if current_input.len() < self.length {
            current_input.push(character);
            state.update_value(&self.input_state, current_input);
        }

        return vec![];
    }
}

impl<Data, App, Text> Element<App> for InputField<Data, App, Text>
where
    Data: for<'a> Selector<'a, App, String> + SafeUnwrap,
    App: Application,
    Text: Display + 'static,
{
    fn get_state(&self) -> &ElementState<App> {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState<App> {
        &mut self.state
    }

    fn resolve(&mut self, state: &Tracker<App>, theme_selector: App::ThemeSelector, placement_resolver: &mut PlacementResolver<App>) {
        let height_bound = *state.get_safe(&InputTheme::height_bound(theme_selector));
        let size_bound = self.width_bound.add_height(height_bound);

        self.state.resolve(placement_resolver, &size_bound);
    }

    fn hovered_element(&self, mouse_position: App::Position, mouse_mode: &App::MouseInputMode) -> HoverInformation<App> {
        match mouse_mode.is_none() {
            true => self.state.hovered_element(mouse_position),
            false => HoverInformation::Missed,
        }
    }

    fn left_click(&mut self, _state: &Context<App>, _update: &mut bool) -> Vec<ClickAction<App>> {
        vec![ClickAction::FocusElement]
    }

    fn input_character(&mut self, state: &Context<App>, character: char) -> (bool, Vec<ClickAction<App>>) {
        (true, match character {
            '\u{8}' | '\u{7f}' => self.remove_character(state),
            '\r' => (self.enter_action)(state),
            character => self.add_character(state, character),
        })
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

        let hovered_element = state.get_safe(&App::HoveredElementSelector::default());
        let focused_element = state.get_safe(&App::FocusedElementSelector::default());

        let input_state = state.get_safe(&self.input_state);
        let is_hovererd = self.is_cell_self(&hovered_element);
        let is_focused = self.is_cell_self(&focused_element);
        let text_offset = *state.get_safe(&InputTheme::text_offset(theme_selector));

        let text = if input_state.is_empty() && !is_focused {
            self.ghost_text.to_string()
        } else if self.hidden {
            input_state.chars().map(|_| '*').collect()
        } else {
            input_state.clone()
        };

        let background_color = if is_hovererd {
            state.get_safe(&InputTheme::hovered_background_color(theme_selector))
        } else if is_focused {
            state.get_safe(&InputTheme::focused_background_color(theme_selector))
        } else {
            state.get_safe(&InputTheme::background_color(theme_selector))
        };

        let text_color = if input_state.is_empty() && !is_focused {
            state.get_safe(&InputTheme::ghost_text_color(theme_selector))
        } else if is_focused {
            state.get_safe(&InputTheme::focused_text_color(theme_selector))
        } else {
            state.get_safe(&InputTheme::text_color(theme_selector))
        };

        renderer.render_background(*state.get_safe(&InputTheme::corner_radius(theme_selector)), *background_color);

        renderer.render_text(
            &text,
            text_offset,
            *text_color,
            *state.get_safe(&InputTheme::font_size(theme_selector)),
        );

        if is_focused {
            let cursor_offset = (text_offset.left() + state.get_safe(&InputTheme::cursor_offset(theme_selector)))
                * state.get_safe(&App::ScaleSelector::default()).get_factor()
                + renderer
                    .get_text_dimensions(&text, *state.get_safe(&InputTheme::font_size(theme_selector)), f32::MAX)
                    .width();

            let cursor_position = App::Position::only_left(cursor_offset);
            let cursor_size = App::Size::new(
                *state.get_safe(&InputTheme::cursor_width(theme_selector)),
                self.state.cached_size.height(),
            );

            renderer.render_rectangle(
                cursor_position,
                cursor_size,
                App::CornerRadius::zero(),
                *state.get_safe(&InputTheme::text_color(theme_selector)),
            );
        }
    }
}
