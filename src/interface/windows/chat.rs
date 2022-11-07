use std::cell::RefCell;
use std::ops::Not;
use std::rc::Rc;

use derive_new::new;
use procedural::*;

use crate::input::UserEvent;
use crate::interface::*;
use crate::network::ChatMessage;

#[derive(new)]
pub struct PrototypeChatWindow {
    messages: Rc<RefCell<Vec<ChatMessage>>>,
}

impl PrototypeChatWindow {
    pub const WINDOW_CLASS: &'static str = "chat";
}

impl PrototypeWindow for PrototypeChatWindow {
    fn window_class(&self) -> Option<&str> {
        PrototypeChatWindow::WINDOW_CLASS.into()
    }

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, avalible_space: Size) -> Window {
        let input_text = Rc::new(RefCell::new(String::new()));

        let button_selector = {
            let input_text = input_text.clone();

            move || !input_text.borrow().is_empty()
        };

        let button_action = {
            let input_text = input_text.clone();

            move || {
                let message: String = input_text.borrow_mut().drain(..).collect();
                Some(ClickAction::Event(UserEvent::SendMessage(message)))
            }
        };

        let input_action = {
            let input_text = input_text.clone();
            Box::new(move || {
                let message: String = input_text.borrow_mut().drain(..).collect();
                message
                    .is_empty()
                    .not()
                    .then_some(ClickAction::Event(UserEvent::SendMessage(message)))
            })
        };

        let elements: Vec<ElementCell> = vec![
            cell!(InputField::<30>::new(
                input_text,
                "write message or command",
                input_action,
                dimension!(75%)
            )) as _,
            Button::default()
                .with_static_text("send")
                .with_disabled_selector(button_selector)
                .with_action_closure(button_action)
                .with_width(dimension!(25%))
                .wrap(),
            cell!(ScrollView::new(
                vec![cell!(Chat::new(self.messages.clone()))],
                constraint!(100%, ?)
            )),
        ];

        Window::new(
            window_cache,
            interface_settings,
            avalible_space,
            "Chat".to_string(),
            Self::WINDOW_CLASS.to_string().into(),
            elements,
            constraint!(200 > 500 < 800, 100 > 100 < 600),
            false,
        )
    }
}
