use std::cell::RefCell;
use std::ops::Not;
use std::rc::Rc;

use derive_new::new;
use procedural::*;

use crate::input::UserEvent;
use crate::interface::*;
use crate::loaders::Service;
use crate::network::LoginSettings;

#[derive(new)]
pub struct LoginWindow {
    service: Service,
    login_settings: LoginSettings,
}

impl LoginWindow {
    pub const WINDOW_CLASS: &'static str = "login";
}

impl PrototypeWindow for LoginWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, available_space: Size) -> Window {
        let username = Rc::new(RefCell::new(self.login_settings.username.clone()));
        let password = Rc::new(RefCell::new(self.login_settings.password.clone()));

        let selector = {
            let username = username.clone();
            let password = password.clone();
            move || !username.borrow().is_empty() && !password.borrow().is_empty()
        };

        let action = {
            let username = username.clone();
            let password = password.clone();
            let service = self.service.clone();

            move || {
                Some(ClickAction::Event(UserEvent::LogIn(
                    service.clone(),
                    username.borrow().clone(),
                    password.borrow().clone(),
                )))
            }
        };

        let username_action = {
            let username = username.clone();
            Box::new(move || {
                username
                    .borrow()
                    .is_empty()
                    .not()
                    .then_some(ClickAction::FocusNext(FocusMode::FocusNext))
            })
        };

        let password_action = {
            let username = username.clone();
            let password = password.clone();
            let service = self.service.clone();

            Box::new(move || match password.borrow().is_empty() {
                _ if username.borrow().is_empty() => Some(ClickAction::FocusNext(FocusMode::FocusPrevious)),
                true => None,
                false => Some(ClickAction::Event(UserEvent::LogIn(
                    service.clone(),
                    username.borrow().clone(),
                    password.borrow().clone(),
                ))),
            })
        };

        let elements = vec![
            InputField::<24>::new(username, "Username", username_action, dimension!(100%)).wrap(),
            InputField::<24, true>::new(password, "Password", password_action, dimension!(100%)).wrap(),
            StateButton::default()
                .with_text("Remember username")
                .with_selector(|state_provider| state_provider.login_settings.remember_username)
                .with_event(UserEvent::ToggleRemeberUsername)
                .with_transparent_background()
                .wrap(),
            StateButton::default()
                .with_text("Remember password")
                .with_selector(|state_provider| state_provider.login_settings.remember_password)
                .with_event(UserEvent::ToggleRemeberPassword)
                .with_transparent_background()
                .wrap(),
            Button::default()
                .with_text("Log in")
                .with_disabled_selector(selector)
                .with_event(Box::new(action))
                .wrap(),
        ];

        WindowBuilder::default()
            .with_title("Log In".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size(constraint!(200 > 250 < 300, ? < 80%))
            .with_elements(elements)
            .build(window_cache, interface_settings, available_space)
    }
}
