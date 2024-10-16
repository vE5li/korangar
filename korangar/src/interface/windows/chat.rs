use korangar_interface::application::Size;
use korangar_interface::components::text_box::DefaultHandler;
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{Element, StateElement};
use korangar_interface::layout::area::Area;
use korangar_interface::layout::{Resolver, WindowLayout};
use korangar_interface::prelude::{HorizontalAlignment, VerticalAlignment};
use korangar_interface::window::{CustomWindow, Window};
use korangar_networking::MessageColor;
use rust_state::{Context, Path, RustState};

use super::WindowClass;
use crate::graphics::Color;
use crate::input::InputEvent;
use crate::loaders::{FontSize, OverflowBehavior};
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::{ChatThemePathExt, InterfaceThemePathExt, InterfaceThemeType};
use crate::state::{ChatMessage, ClientState, ClientStatePathExt, client_state, client_theme};

const MAXIMUM_CHAT_MESSAGE_LENGTH: usize = 80;

/// ZST for getting the focus id of the chat text box. This is only needed to
/// focus the chat when pressing enter.
pub struct ChatTextBox;

struct ChatLayoutInfo {
    area: Area,
    // TODO: Don't allocate this every frame.
    message_heights: Vec<f32>,
}

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
    type LayoutInfo = ChatLayoutInfo;

    fn create_layout_info(
        &mut self,
        state: &Context<ClientState>,
        _: ElementStoreMut<'_>,
        resolver: &mut Resolver<'_, ClientState>,
    ) -> Self::LayoutInfo {
        let chat_messages = state.get(&self.chat_messages_path);
        // TODO: Theme this.
        let message_spacing = 5.0;

        let mut total_height = 0.0;
        let message_heights = chat_messages
            .iter()
            .map(|chat_message| {
                let (size, _) = resolver.get_text_dimensions(
                    &chat_message.text,
                    // TODO: Theme this.
                    FontSize(14.0),
                    HorizontalAlignment::Left { offset: 5.0, border: 3.0 },
                    OverflowBehavior::LineBreak,
                );

                if total_height != 0.0 {
                    total_height += message_spacing;
                }

                total_height += size.height();

                size.height()
            })
            .collect();

        let area = resolver.with_height(total_height);

        Self::LayoutInfo { area, message_heights }
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a Context<ClientState>,
        _: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        let chat_messages = state.get(&self.chat_messages_path);
        // TODO: Theme this.
        let message_spacing = 5.0;

        let mut offset = 0.0;
        chat_messages
            .iter()
            .zip(layout_info.message_heights.iter())
            .for_each(|(chat_message, message_height)| {
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

                if offset != 0.0 {
                    offset += message_spacing;
                }

                let text_area = Area {
                    left: layout_info.area.left,
                    top: layout_info.area.top + offset,
                    width: layout_info.area.width,
                    height: *message_height,
                };

                layout.add_text(
                    text_area,
                    &chat_message.text,
                    // TODO: Theme this.
                    FontSize(14.0),
                    color,
                    HorizontalAlignment::Left { offset: 5.0, border: 3.0 },
                    VerticalAlignment::Center { offset: 0.0 },
                    OverflowBehavior::LineBreak,
                );

                offset += message_height;
            });
    }
}

/// Internal state of the chat window.
#[derive(Default, RustState, StateElement)]
pub struct ChatWindowState {
    current_text: String,
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

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let current_text_path = self.chat_window_state.current_text();
        let send_action = move |state: &Context<ClientState>, queue: &mut EventQueue<ClientState>| {
            let text = state.get(&current_text_path);

            if !text.is_empty() {
                // Clear the text box.
                state.update_value_with(current_text_path, |current_text| current_text.clear());
                queue.queue(InputEvent::SendMessage { text: text.clone() });
                queue.queue(Event::Unfocus);
            }
        };

        window! {
            title: client_state().localization().chat_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::Game,
            background_color: client_theme().chat().window_color(),
            resizable: true,
            border: 3.0,
            gaps: 2.0,
            title_gap: 0.0,
            minimum_height: 150.0,
            maximum_height: 800.0,
            elements: (
                text_box! {
                    ghost_text: client_state().localization().chat_text_box_message(),
                    state: current_text_path,
                    input_handler: DefaultHandler::<_, _, MAXIMUM_CHAT_MESSAGE_LENGTH>::new(current_text_path, send_action),
                    background_color: client_theme().chat().text_box_background_color(),
                    focused_background_color: Color::rgba(0.0, 0.0, 0.0, 0.8),
                    focus_id: ChatTextBox,
                    // TODO:
                    // follow: true,
                },
                scroll_view! {
                    children: (
                        ChatElement::new(self.chat_messages_path),
                    ),
                },
            ),
        }
    }
}
