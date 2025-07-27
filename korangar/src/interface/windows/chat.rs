use std::sync::Arc;

use korangar_interface::components::text_box::DefaultHandler;
use korangar_interface::element::id::ElementIdGenerator;
use korangar_interface::element::store::ElementStore;
use korangar_interface::element::{DefaultLayoutInfo, Element, StateElement};
use korangar_interface::event::{ClickAction, Event, EventQueue};
use korangar_interface::layout::area::Area;
use korangar_interface::layout::{Layout, Resolver};
use korangar_interface::prelude::{HorizontalAlignment, VerticalAlignment};
use korangar_interface::window::{CustomWindow, StateWindow, Window, WindowTrait};
use korangar_networking::MessageColor;
use rust_state::{Context, Path};

use super::WindowClass;
use crate::graphics::Color;
use crate::input::UserEvent;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::WindowCache;
use crate::loaders::{FontLoader, FontSize};
use crate::state::{
    ChatMessage, ChatThemePathExt, ChatWindowState, ChatWindowStatePathExt, ClientState, ClientThemeType, InterfaceThemePathExt,
    client_theme,
};

const MAXIMUM_CHAT_MESSAGE_LENGTH: usize = 80;

/// ZST for getting the focus id of the chat text box. This is only needed to
/// focus the chat when pressing enter.
pub struct ChatTextBox;

struct ChatElement<A> {
    chat_messages_path: A,
}

impl<A> ChatElement<A> {
    fn new(chat_messages_path: A) -> Self {
        Self { chat_messages_path }
    }
}

impl<A> Element<ClientState> for ChatElement<A>
where
    A: Path<ClientState, Vec<ChatMessage>>,
{
    fn create_layout_info(
        &mut self,
        state: &Context<ClientState>,
        store: &mut ElementStore,
        generator: &mut ElementIdGenerator,
        resolver: &mut Resolver,
    ) -> Self::LayoutInfo {
        let chat_messages = state.get(&self.chat_messages_path);

        // The lower part of the last text gets cut off without this.
        // NOTE: This might be due to estimating the the text height, can likely be
        // removed after getting the correct dimensions.
        const PADDING: f32 = 5.0;

        let total_height = PADDING
            + chat_messages
                .iter()
                .map(|chat_message| {
                    // FIX: This should get the text dimensions.
                    16.0
                })
                .sum::<f32>();

        let area = resolver.with_height(total_height);

        DefaultLayoutInfo { area }
    }

    fn layout_element<'a>(
        &'a self,
        state: &'a Context<ClientState>,
        store: &'a ElementStore,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut Layout<'a, ClientState>,
    ) {
        let chat_messages = state.get(&self.chat_messages_path);

        let mut offset = 0.0;
        chat_messages.iter().for_each(|chat_message| {
            let color = match chat_message.color {
                MessageColor::Rgb { red, green, blue } => Color::rgb_u8(red, green, blue),
                // TODO: Make the color right.
                MessageColor::Broadcast => Color::monochrome_u8(255),
                // TODO: Make the color right.
                MessageColor::Server => Color::monochrome_u8(255),
                // TODO: Make the color right.
                MessageColor::Error => Color::monochrome_u8(255),
                // TODO: Make the color right.
                MessageColor::Information => Color::monochrome_u8(255),
            };

            let text_area = Area {
                left: layout_info.area.left,
                top: layout_info.area.top + offset,
                width: layout_info.area.width,
                height: 20.0,
            };

            offset += 16.0;

            layout.add_text(
                text_area,
                &chat_message.text,
                FontSize(14.0),
                color,
                HorizontalAlignment::Left { offset: 5.0 },
                VerticalAlignment::Center { offset: 0.0 },
            );
        });
    }
}

pub struct ChatWindow<A, B> {
    chat_window_state: A,
    chat_messages_path: B,
}

impl<A, B> ChatWindow<A, B> {
    pub fn new(chat_window_state: A, chat_messages_path: B) -> Self {
        Self {
            chat_window_state,
            chat_messages_path,
        }
    }
}

impl<A, B> CustomWindow<ClientState> for ChatWindow<A, B>
where
    A: Path<ClientState, ChatWindowState>,
    B: Path<ClientState, Vec<ChatMessage>>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Chat)
    }

    fn to_window<'a>(self) -> impl WindowTrait<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let current_message_path = self.chat_window_state.current_message();
        let send_action = move |state: &Context<ClientState>, queue: &mut EventQueue<ClientState>| {
            let message = state.get(&current_message_path);

            if !message.is_empty() {
                // Clear the text box.
                state.update_value_with(current_message_path, |current_message| current_message.clear());
                queue.queue(UserEvent::SendMessage(message.clone()));
                queue.queue(Event::Unfocus);
            }
        };

        window! {
            title: "Chat",
            class: Self::window_class(),
            theme: ClientThemeType::Game,
            background_color: client_theme().chat().window_color(),
            minimum_height: 150.0,
            maximum_height: 800.0,
            elements: (
                text_box! {
                    ghost_text: "Enter chat message or command",
                    state: current_message_path,
                    input_handler: DefaultHandler::<_, _, MAXIMUM_CHAT_MESSAGE_LENGTH>::new(current_message_path, send_action),
                    background_color: client_theme().chat().text_box_background_color(),
                    focus_id: ChatTextBox,
                    // TODO:
                    // follow: true,
                },
                scroll_view! {
                    children: (
                        ChatElement::new(self.chat_messages_path),
                    ),
                    height_bound: HeightBound::WithMax,
                },
            ),
        }
    }
}
