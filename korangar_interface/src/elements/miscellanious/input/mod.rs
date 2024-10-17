mod builder;

use std::fmt::Display;

pub use self::builder::InputFieldBuilder;
use crate::application::{
    Application, CornerRadiusTraitExt, MouseInputModeTrait, PositionTrait, PositionTraitExt, ScalingTrait, SizeTrait,
};
use crate::elements::{Element, ElementState};
use crate::event::{ChangeEvent, ClickAction, HoverInformation};
use crate::layout::{DimensionBound, PlacementResolver};
use crate::state::{PlainTrackedState, TrackedState, ValueState};
use crate::theme::{InputTheme, InterfaceTheme};

/// Local type alias to simplify the builder.
type EnterAction<App> = Box<dyn FnMut() -> Vec<ClickAction<App>>>;

pub struct InputField<App, Text>
where
    App: Application,
    Text: Display + 'static,
{
    input_state: PlainTrackedState<String>,
    ghost_text: Text,
    enter_action: EnterAction<App>,
    length: usize,
    hidden: bool,
    width_bound: DimensionBound,
    state: ElementState<App>,
}

impl<App, Text> InputField<App, Text>
where
    App: Application,
    Text: Display + 'static,
{
    fn remove_character(&mut self) -> Vec<ClickAction<App>> {
        self.input_state.with_mut(|input_state| {
            if input_state.is_empty() {
                return ValueState::Unchanged(Vec::new());
            }

            input_state.pop();

            ValueState::Mutated(vec![ClickAction::ChangeEvent(ChangeEvent::RENDER_WINDOW)])
        })
    }

    fn add_character(&mut self, character: char) -> Vec<ClickAction<App>> {
        self.input_state.with_mut(|input_state| {
            if input_state.len() >= self.length {
                return ValueState::Unchanged(Vec::new());
            }

            input_state.push(character);

            ValueState::Mutated(vec![ClickAction::ChangeEvent(ChangeEvent::RENDER_WINDOW)])
        })
    }
}

impl<App, Text> Element<App> for InputField<App, Text>
where
    App: Application,
    Text: Display + 'static,
{
    fn get_state(&self) -> &ElementState<App> {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState<App> {
        &mut self.state
    }

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver<App>, _application: &App, theme: &App::Theme) {
        let size_bound = self.width_bound.add_height(theme.input().height_bound());
        self.state.resolve(placement_resolver, &size_bound);
    }

    fn hovered_element(&self, mouse_position: App::Position, mouse_mode: &App::MouseInputMode) -> HoverInformation<App> {
        match mouse_mode.is_none() {
            true => self.state.hovered_element(mouse_position),
            false => HoverInformation::Missed,
        }
    }

    fn left_click(&mut self, _update: &mut bool) -> Vec<ClickAction<App>> {
        vec![ClickAction::FocusElement]
    }

    fn input_character(&mut self, character: char) -> (bool, Vec<ClickAction<App>>) {
        (true, match character {
            '\u{8}' | '\u{7f}' => self.remove_character(),
            '\r' => (self.enter_action)(),
            character => self.add_character(character),
        })
    }

    fn render(
        &self,
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
        let mut renderer = self.state.element_renderer(renderer, application, parent_position, screen_clip);

        let input_state = self.input_state.get();
        let is_hovererd = self.is_element_self(hovered_element);
        let is_focused = self.is_element_self(focused_element);
        let text_offset = theme.input().text_offset();

        let text = if input_state.is_empty() && !is_focused {
            self.ghost_text.to_string()
        } else if self.hidden {
            input_state.chars().map(|_| '*').collect()
        } else {
            input_state.clone()
        };

        let background_color = if is_hovererd {
            theme.input().hovered_background_color()
        } else if is_focused {
            theme.input().focused_background_color()
        } else {
            theme.input().background_color()
        };

        let text_color = if input_state.is_empty() && !is_focused {
            theme.input().ghost_text_color()
        } else if is_focused {
            theme.input().focused_text_color()
        } else {
            theme.input().text_color()
        };

        renderer.render_background(theme.input().corner_radius(), background_color);
        renderer.render_text(&text, text_offset, text_color, theme.input().font_size());

        if is_focused {
            let cursor_offset = (text_offset.left() + theme.input().cursor_offset()) * application.get_scaling().get_factor()
                + renderer.get_text_dimensions(&text, theme.input().font_size(), f32::MAX).width();

            let cursor_position = App::Position::only_left(cursor_offset);
            let cursor_size = App::Size::new(theme.input().cursor_width(), self.state.cached_size.height());

            renderer.render_rectangle(
                cursor_position,
                cursor_size,
                App::CornerRadius::zero(),
                theme.input().text_color(),
            );
        }
    }
}
