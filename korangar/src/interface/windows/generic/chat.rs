use std::cell::RefCell;
use std::rc::Rc;

use derive_new::new;
use korangar_interface::elements::{ButtonBuilder, ElementWrap, InputFieldBuilder, ScrollView};
use korangar_interface::event::ClickAction;
use korangar_interface::state::{PlainRemote, PlainTrackedState, TrackedState, TrackedStateTake};
use korangar_interface::windows::{PrototypeWindow, Window, WindowBuilder};
use korangar_procedural::{dimension_bound, size_bound};

use crate::input::UserEvent;
use crate::interface::application::InterfaceSettings;
use crate::interface::elements::ChatBuilder;
use crate::interface::layout::ScreenSize;
use crate::interface::theme::InterfaceTheme;
use crate::interface::windows::WindowCache;
use crate::loaders::FontLoader;
use crate::network::ChatMessage;

#[derive(new)]
pub struct ChatWindow {
    messages: PlainRemote<Vec<ChatMessage>>,
    font_loader: Rc<RefCell<FontLoader>>,
}

impl ChatWindow {
    pub const WINDOW_CLASS: &'static str = "chat";
}

impl PrototypeWindow<InterfaceSettings> for ChatWindow {
    fn window_class(&self) -> Option<&str> {
        ChatWindow::WINDOW_CLASS.into()
    }

    fn to_window(
        &self,
        window_cache: &WindowCache,
        application: &InterfaceSettings,
        available_space: ScreenSize,
    ) -> Window<InterfaceSettings> {
        let input_text = PlainTrackedState::<String>::default();

        let button_selector = {
            let input_text = input_text.clone();

            move || !input_text.get().is_empty()
        };

        let button_action = {
            let mut input_text = input_text.clone();

            move || {
                let message = input_text.take();
                vec![ClickAction::Custom(UserEvent::SendMessage(message))]
            }
        };

        let input_action = {
            let mut input_text = input_text.clone();
            Box::new(move || {
                let message = input_text.take();

                (!message.is_empty())
                    .then_some(vec![ClickAction::Custom(UserEvent::SendMessage(message))])
                    .unwrap_or_default()
            })
        };

        let elements = vec![
            InputFieldBuilder::new()
                .with_state(input_text)
                .with_ghost_text("Write message or command")
                .with_enter_action(input_action)
                .with_length(30)
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
                        .with_messages(self.messages.clone())
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
            .with_background_color(Box::new(|theme: &InterfaceTheme| theme.chat.background_color.get()))
            .with_elements(elements)
            .build(window_cache, application, available_space)
    }
}
