use std::cell::RefCell;
use std::ops::Not;
use std::rc::Rc;

use derive_new::new;
use procedural::*;

use crate::input::UserEvent;
use crate::interface::{ElementCell, FramedWindow, InterfaceSettings, PrototypeWindow, Size, Window, WindowCache, *};
use crate::network::LoginSettings;

#[derive(new)]
pub struct LoginWindow {
    login_settings: LoginSettings,
}

impl LoginWindow {

    pub const WINDOW_CLASS: &'static str = "login";
}

impl PrototypeWindow for LoginWindow {

    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(
        &self,
        window_cache: &WindowCache,
        interface_settings: &InterfaceSettings,
        avalible_space: Size,
    ) -> Box<dyn Window + 'static> {

        let username = Rc::new(RefCell::new(self.login_settings.username.clone()));
        let password = Rc::new(RefCell::new(self.login_settings.password.clone()));

        let selector = {

            let username = username.clone();
            let password = password.clone();
            Box::new(move || !username.borrow().is_empty() && !password.borrow().is_empty())
        };

        let action = {

            let username = username.clone();
            let password = password.clone();
            Box::new(move || UserEvent::LogIn(username.borrow().clone(), password.borrow().clone()))
        };

        let username_action = {

            let username = username.clone();
            Box::new(move || /*username.borrow().is_empty().not().then_some(ChangeEvent::FocusNext)*/ None)
        };

        let password_action = {

            let username = username.clone();
            let password = password.clone();

            Box::new(move || {
                match password.borrow().is_empty() {
                    _ if username.borrow().is_empty() => None, //Some(ChangeEvent::FocusPrevious),
                    true => None,
                    false => None, //Some(ChangeEvent::LeftClickNext),
                }
            })
        };

        let elements: Vec<ElementCell> = vec![
            cell!(InputField::<24>::new(username, "username", username_action)),
            cell!(InputField::<24, true>::new(password, "password", password_action)),
            cell!(StateButton::new(
                "remember username",
                UserEvent::ToggleRemeberUsername,
                Box::new(|state_provider| state_provider.login_settings.remember_username)
            )),
            cell!(StateButton::new(
                "remember password",
                UserEvent::ToggleRemeberPassword,
                Box::new(|state_provider| state_provider.login_settings.remember_password)
            )),
            cell!(FormButton::new("log in", selector, action)),
        ];

        Box::from(FramedWindow::new(
            window_cache,
            interface_settings,
            avalible_space,
            "Log In".to_string(),
            Self::WINDOW_CLASS.to_string().into(),
            elements,
            constraint!(200 > 250 < 300, ? < 80%),
        ))
    }
}
