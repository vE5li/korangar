use std::cell::RefCell;
use std::ops::Not;
use std::rc::Rc;

use derive_new::new;
use procedural::*;

use crate::input::UserEvent;
use crate::interface::*;
use crate::loaders::ClientInfo;
use crate::network::LoginSettings;

#[derive(new)]
pub struct LoginWindow<'a> {
    client_info: &'a ClientInfo,
}

impl<'a> LoginWindow<'a> {
    pub const WINDOW_CLASS: &'static str = "login";
}

impl<'a> PrototypeWindow for LoginWindow<'a> {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, available_space: ScreenSize) -> Window {
        let mut login_settings = LoginSettings::new();

        let options = self
            .client_info
            .services
            .iter()
            .map(|service| (service.display_name.clone().unwrap(), service.service_id()))
            .collect();

        // FIX: This will panic when no services are present. What is the correct
        // behavior?
        let selected_service = login_settings
            .recent_service_id
            // Only use the recent server if it is still in the client info
            .filter(|&recent_service_id| {
                self.client_info
                    .services
                    .iter()
                    .any(|service| service.service_id() == recent_service_id)
            })
            .unwrap_or_else(|| self.client_info.services[0].service_id());

        let saved_settings = login_settings.service_settings.entry(selected_service).or_default();

        let username = Rc::new(RefCell::new(saved_settings.username.clone()));
        let password = Rc::new(RefCell::new(saved_settings.password.clone()));

        let selected_service = TrackedState::new(selected_service);
        let login_settings = Rc::new(RefCell::new(login_settings));

        let selector = {
            let username = username.clone();
            let password = password.clone();
            move || !username.borrow().is_empty() && !password.borrow().is_empty()
        };

        let service_changed = {
            let username = username.clone();
            let password = password.clone();
            let login_settings = login_settings.clone();
            let selected_service = selected_service.clone();

            Box::new(move || {
                let service_id = selected_service.get();
                let login_settings = login_settings.borrow_mut();

                if let Some(saved_settings) = login_settings.service_settings.get(&service_id) {
                    *username.borrow_mut() = saved_settings.username.clone();
                    *password.borrow_mut() = saved_settings.password.clone();
                }

                Vec::new()
            })
        };

        let login_action = {
            let username = username.clone();
            let password = password.clone();
            let login_settings = login_settings.clone();
            let selected_service = selected_service.clone();

            move || {
                // TODO: Deduplicate code
                let service_id = selected_service.get();

                let mut login_settings = login_settings.borrow_mut();
                login_settings.recent_service_id = Some(service_id);

                let saved_settings = login_settings.service_settings.entry(service_id).or_default();
                saved_settings.username = username.borrow().clone();
                saved_settings.password = password.borrow().clone();

                vec![ClickAction::Event(UserEvent::LogIn {
                    service_id: selected_service.get(),
                    username: username.borrow().clone(),
                    password: password.borrow().clone(),
                })]
            }
        };

        let username_action = {
            let username = username.clone();
            Box::new(move || {
                username
                    .borrow()
                    .is_empty()
                    .not()
                    .then_some(vec![ClickAction::FocusNext(FocusMode::FocusNext)])
                    .unwrap_or_default()
            })
        };

        let password_action = {
            let username = username.clone();
            let password = password.clone();
            let login_settings = login_settings.clone();
            let selected_service = selected_service.clone();

            Box::new(move || match password.borrow().is_empty() {
                _ if username.borrow().is_empty() => vec![ClickAction::FocusNext(FocusMode::FocusPrevious)],
                true => Vec::new(),
                false => {
                    // TODO: Deduplicate code
                    let service_id = selected_service.get();

                    let mut login_settings = login_settings.borrow_mut();
                    login_settings.recent_service_id = Some(service_id);

                    let saved_settings = login_settings.service_settings.entry(service_id).or_default();
                    saved_settings.username = username.borrow().clone();
                    saved_settings.password = password.borrow().clone();

                    vec![ClickAction::Event(UserEvent::LogIn {
                        service_id: selected_service.get(),
                        username: username.borrow().clone(),
                        password: password.borrow().clone(),
                    })]
                }
            })
        };

        let remember_username_selector = {
            let login_settings = login_settings.clone();
            let selected_service = selected_service.clone();

            move |_: &StateProvider| {
                let service_id = selected_service.get();
                let mut login_settings = login_settings.borrow_mut();
                let saved_settings = login_settings.service_settings.entry(service_id).or_default();

                saved_settings.remember_username
            }
        };

        let remember_password_selector = {
            let login_settings = login_settings.clone();
            let selected_service = selected_service.clone();

            move |_: &StateProvider| {
                let service_id = selected_service.get();
                let mut login_settings = login_settings.borrow_mut();
                let saved_settings = login_settings.service_settings.entry(service_id).or_default();

                saved_settings.remember_password
            }
        };

        let remember_username_action = {
            let login_settings = login_settings.clone();
            let selected_service = selected_service.clone();

            Box::new(move || {
                let service_id = selected_service.get();
                let mut login_settings = login_settings.borrow_mut();
                let saved_settings = login_settings.service_settings.entry(service_id).or_default();

                saved_settings.remember_username = !saved_settings.remember_username;

                Vec::new()
            })
        };

        let remember_password_action = {
            let login_settings = login_settings.clone();
            let selected_service = selected_service.clone();

            Box::new(move || {
                let service_id = selected_service.get();
                let mut login_settings = login_settings.borrow_mut();
                let saved_settings = login_settings.service_settings.entry(service_id).or_default();

                saved_settings.remember_password = !saved_settings.remember_password;

                Vec::new()
            })
        };

        let elements = vec![
            Text::default().with_text("Select service").wrap(),
            PickList::default()
                .with_options(options)
                .with_selected(selected_service)
                .with_event(service_changed)
                .wrap(),
            Text::default().with_text("Account data").wrap(),
            InputField::<24>::new(username, "Username", username_action, dimension_bound!(100%)).wrap(),
            InputField::<24, true>::new(password, "Password", password_action, dimension_bound!(100%)).wrap(),
            Container::new({
                vec![
                    StateButton::default()
                        .with_text("Remember username")
                        .with_selector(remember_username_selector)
                        .with_event(remember_username_action)
                        .with_transparent_background()
                        .wrap(),
                    StateButton::default()
                        .with_text("Remember password")
                        .with_selector(remember_password_selector)
                        .with_event(remember_password_action)
                        .with_transparent_background()
                        .wrap(),
                ]
            })
            .wrap(),
            Button::default()
                .with_text("Log in")
                .with_disabled_selector(selector)
                .with_event(Box::new(login_action))
                .wrap(),
        ];

        WindowBuilder::new()
            .with_title("Log In".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size_bound(SizeBound::DEFAULT_UNBOUNDED)
            .with_elements(elements)
            .with_theme_kind(ThemeKind::Menu)
            .build(window_cache, interface_settings, available_space)
    }
}
