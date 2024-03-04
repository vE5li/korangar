use std::cell::RefCell;
use std::rc::Rc;

use derive_new::new;
use procedural::*;

use crate::input::UserEvent;
use crate::interface::*;
use crate::loaders::FontLoader;
use crate::network::ChatMessage;

#[derive(new)]
pub struct ChatWindow {
    messages: Rc<RefCell<Vec<ChatMessage>>>,
    font_loader: Rc<RefCell<FontLoader>>,
}

impl ChatWindow {
    pub const WINDOW_CLASS: &'static str = "chat";
}

impl PrototypeWindow for ChatWindow {
    fn window_class(&self) -> Option<&str> {
        ChatWindow::WINDOW_CLASS.into()
    }

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, available_space: ScreenSize) -> Window {
        let input_text = Rc::new(RefCell::new(String::new()));

        let button_selector = {
            let input_text = input_text.clone();

            move || !input_text.borrow().is_empty()
        };

        let button_action = {
            let input_text = input_text.clone();

            move || {
                let message: String = input_text.borrow_mut().drain(..).collect();
                vec![ClickAction::Event(UserEvent::SendMessage(message))]
            }
        };

        let input_action = {
            let input_text = input_text.clone();
            Box::new(move || {
                let message: String = input_text.borrow_mut().drain(..).collect();
                (!message.is_empty())
                    .then_some(vec![ClickAction::Event(UserEvent::SendMessage(message))])
                    .unwrap_or_default()
            })
        };

        let elements = vec![
            InputField::<30>::new(input_text, "write message or command", input_action, dimension_bound!(75%)).wrap(),
            Button::default()
                .with_text("send")
                .with_disabled_selector(button_selector)
                .with_event(Box::new(button_action))
                .with_width(dimension_bound!(25%))
                .wrap(),
            ScrollView::new(
                vec![Chat::new(self.messages.clone(), self.font_loader.clone()).wrap()],
                size_bound!(100%, ?),
            )
            .wrap(),
        ];

        WindowBuilder::default()
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size(size_bound!(200 > 500 < 800, 100 > 100 < 600))
            .with_background_color(Box::new(|theme| theme.chat.background_color.get()))
            .with_elements(elements)
            .build(window_cache, interface_settings, available_space)
    }
}
