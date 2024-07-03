use std::cell::RefCell;
use std::rc::Rc;

use derive_new::new;
use korangar_interface::elements::{ButtonBuilder, ElementWrap, InputFieldBuilder, ScrollView};
use korangar_interface::event::ClickAction;
use korangar_interface::state::{PlainRemote, PlainTrackedState, TrackedState, TrackedStateTake};
use korangar_interface::windows::{PrototypeWindow, Window, WindowBuilder};
use korangar_interface::{dimension_bound, size_bound};
use korangar_networking::MessageColor;
use rust_state::{Context, SafeUnwrap, Selector, Tracker};

use crate::input::UserEvent;
use crate::interface::elements::ChatBuilder;
use crate::interface::layout::ScreenSize;
use crate::interface::theme::{ChatTheme, InterfaceTheme};
use crate::interface::windows::WindowCache;
use crate::loaders::FontLoader;
use crate::GameState;

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub text: String,
    pub color: MessageColor,
}

#[derive(new)]
pub struct ChatWindow {
    font_loader: Rc<RefCell<FontLoader>>,
}

impl ChatWindow {
    pub const WINDOW_CLASS: &'static str = "chat";
}

impl PrototypeWindow<GameState> for ChatWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(&self, window_cache: &WindowCache, application: &Context<GameState>, available_space: ScreenSize) -> Window<GameState> {
        let button_selector = |state: &Tracker<GameState>| !state.get_safe(&GameState::chat_input()).is_empty();

        let button_action = |state: &Context<GameState>| {
            let message = state.get_safe(&GameState::chat_input()).clone();
            state.update_value(&GameState::chat_input(), String::new());

            vec![ClickAction::Custom(UserEvent::SendMessage(message))]
        };

        let input_action = Box::new(move |state: &Context<GameState>| {
            let message = state.get_safe(&GameState::chat_input()).clone();
            state.update_value(&GameState::chat_input(), String::new());

            (!message.is_empty())
                .then_some(vec![ClickAction::Custom(UserEvent::SendMessage(message))])
                .unwrap_or_default()
        });

        let elements = vec![
            InputFieldBuilder::new()
                .with_state(GameState::chat_input())
                .with_ghost_text("Write message or command")
                .with_enter_action(input_action)
                .with_length(80)
                .with_width_bound(dimension_bound!(75%))
                .build()
                .wrap(),
            ButtonBuilder::new()
                .with_text("Send")
                .with_disabled_selector(button_selector)
                .with_event(Box::new(button_action))
                .with_width_bound(dimension_bound!(25%))
                .build()
                .wrap(),
            ScrollView::new(
                vec![
                    ChatBuilder::new()
                        .with_messages(GameState::chat_messages())
                        .with_font_loader(self.font_loader.clone())
                        .build()
                        .wrap(),
                ],
                size_bound!(100%, !),
            )
            .wrap(),
        ];

        WindowBuilder::new()
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size_bound(size_bound!(200 > 500 < 800, 100 > 100 < 600))
            .with_background_color(Box::new(|state, theme_selector| {
                *state.get_safe(&ChatTheme::background_color(theme_selector))
            }))
            .with_elements(elements)
            .build(window_cache, application, available_space)
    }
}
