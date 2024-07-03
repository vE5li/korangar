mod builder;

use std::cell::RefCell;
use std::rc::Rc;

use korangar_interface::application::{Application, FontSizeTraitExt, ScalingTrait};
use korangar_interface::elements::{Element, ElementState};
use korangar_interface::event::ChangeEvent;
use korangar_interface::layout::{Dimension, PlacementResolver};
use korangar_interface::size_bound;
use korangar_interface::state::{PlainRemote, Remote};
use rust_state::{SafeUnwrap, Selector, Tracker};

pub use self::builder::ChatBuilder;
use crate::graphics::{Color, InterfaceRenderer, Renderer};
use crate::interface::application::ThemeSelector2;
use crate::interface::layout::{ScreenClip, ScreenPosition};
use crate::interface::theme::ChatTheme;
use crate::interface::windows::ChatMessage;
use crate::loaders::FontLoader;
use crate::{GameState, GameStateScalePath};

pub struct Chat<Messages> {
    messages: Messages,
    font_loader: Rc<RefCell<FontLoader>>,
    state: ElementState<GameState>,
}

impl<Messages> Element<GameState> for Chat<Messages>
where
    Messages: for<'a> Selector<'a, GameState, Vec<ChatMessage>> + SafeUnwrap,
{
    fn get_state(&self) -> &ElementState<GameState> {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState<GameState> {
        &mut self.state
    }

    fn is_focusable(&self) -> bool {
        false
    }

    fn resolve(
        &mut self,
        state: &Tracker<GameState>,
        theme_selector: ThemeSelector2,
        placement_resolver: &mut PlacementResolver<GameState>,
    ) {
        let mut size_bound = size_bound!(100%, 0);
        // Not sure why but 0.0 cuts off the lower part of the text, so add some
        // padding.
        let scale = state.get_safe(&GameStateScalePath::default()).get_factor();
        let mut height = 5.0 * scale;

        // Dividing by the scaling is done to counteract the scaling being applied
        // twice per message. It's not the cleanest solution but it works.
        for message in state.get_safe(&self.messages).iter() {
            height += self
                .font_loader
                .borrow()
                .get_text_dimensions(
                    &message.text,
                    state.get_safe(&ChatTheme::font_size(theme_selector)).scaled(scale),
                    placement_resolver.get_available().width,
                )
                .height
                / scale;
        }

        size_bound.height = Dimension::Absolute(height);
        self.state.resolve(placement_resolver, &size_bound);
    }

    fn update(&mut self) -> Option<ChangeEvent> {
        self.messages.consume_changed().then_some(ChangeEvent::RESOLVE_WINDOW)
    }

    fn render(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        renderer: &InterfaceRenderer,
        application: &Tracker<GameState>,
        theme_selector: ThemeSelector2,
        parent_position: ScreenPosition,
        screen_clip: ScreenClip,
        _second_theme: bool,
    ) {
        let mut renderer = self
            .state
            .element_renderer(render_target, renderer, application, parent_position, screen_clip);

        let mut offset = 0.0;

        for message in application.get_safe(&self.messages).iter() {
            let text = &message.text;
            let font_size = *application.get_safe(&ChatTheme::font_size(theme_selector));

            renderer.render_text(
                text,
                ScreenPosition {
                    left: 0.2,
                    top: offset + 0.2,
                },
                Color::monochrome_u8(0),
                font_size,
            );

            let message_color = match message.color {
                korangar_networking::MessageColor::Rgb { red, green, blue } => Color::rgb_u8(red, green, blue),
                korangar_networking::MessageColor::Broadcast => *application.get_safe(&ChatTheme::broadcast_color(theme_selector)),
                korangar_networking::MessageColor::Server => *application.get_safe(&ChatTheme::server_color(theme_selector)),
                korangar_networking::MessageColor::Error => *application.get_safe(&ChatTheme::error_color(theme_selector)),
                korangar_networking::MessageColor::Information => *application.get_safe(&ChatTheme::information_color(theme_selector)),
            };

            // Dividing by the scaling is done to counteract the scaling being applied
            // twice per message. It's not the cleanest solution but it works.
            offset +=
                renderer.render_text(text, ScreenPosition::only_top(offset), message_color, font_size) / application.get_scaling_factor();
        }
    }
}
