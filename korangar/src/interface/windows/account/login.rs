use std::ops::Not;

use derive_new::new;
use korangar_interface::elements::{
    ButtonBuilder, Container, ElementWrap, FocusMode, InputFieldBuilder, PickList, StateButtonBuilder, Text,
};
use korangar_interface::event::ClickAction;
use korangar_interface::size_bound;
use korangar_interface::state::{PlainTrackedState, TrackedState, TrackedStateBinary, TrackedStateClone, TrackedStateExt};
use korangar_interface::windows::{PrototypeWindow, Window, WindowBuilder};

use crate::input::UserEvent;
use crate::interface::application::InterfaceSettings;
use crate::interface::layout::ScreenSize;
use crate::interface::theme::InterfaceThemeKind;
use crate::interface::windows::WindowCache;
use crate::loaders::ClientInfo;
use crate::network::LoginSettings;

#[derive(new)]
pub struct LoginWindow<'a> {
    client_info: &'a ClientInfo,
}

impl<'a> LoginWindow<'a> {
    pub const WINDOW_CLASS: &'static str = "login";
}

impl<'a> PrototypeWindow<InterfaceSettings> for LoginWindow<'a> {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(
        &self,
        window_cache: &WindowCache,
        application: &InterfaceSettings,
        available_space: ScreenSize,
    ) -> Window<InterfaceSettings> {
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

        let username = PlainTrackedState::new(saved_settings.username.clone());
        let password = PlainTrackedState::new(saved_settings.password.clone());

        let selected_service = PlainTrackedState::new(selected_service);
        let login_settings = PlainTrackedState::new(login_settings);

        let selector = {
            let username = username.clone();
            let password = password.clone();
            move || !username.get().is_empty() && !password.get().is_empty()
        };

        let service_changed = {
            let mut username = username.clone();
            let mut password = password.clone();
            let mut login_settings = login_settings.clone();
            let selected_service = selected_service.clone();

            Box::new(move || {
                let service_id = selected_service.cloned();
                let saved_settings =
                    login_settings.mutate(|login_settings| login_settings.service_settings.entry(service_id).or_default().clone());

                username.mutate(|username| {
                    *username = saved_settings.username;
                });
                password.mutate(|password| {
                    *password = saved_settings.password;
                });

                Vec::new()
            })
        };

        let login_action = {
            let username = username.clone();
            let password = password.clone();
            let mut login_settings = login_settings.clone();
            let selected_service = selected_service.clone();

            move || {
                // TODO: Deduplicate code
                let service_id = selected_service.cloned();

                login_settings.mutate(|login_settings| {
                    login_settings.recent_service_id = Some(service_id);

                    let saved_settings = login_settings.service_settings.entry(service_id).or_default();
                    saved_settings.username = username.cloned();
                    saved_settings.password = password.cloned();
                });

                vec![ClickAction::Custom(UserEvent::LogIn {
                    service_id: selected_service.cloned(),
                    username: username.cloned(),
                    password: password.cloned(),
                })]
            }
        };

        let username_action = {
            let username = username.clone();
            Box::new(move || {
                username
                    .get()
                    .is_empty()
                    .not()
                    .then_some(vec![ClickAction::FocusNext(FocusMode::FocusNext)])
                    .unwrap_or_default()
            })
        };

        let password_action = {
            let username = username.clone();
            let password = password.clone();
            let mut login_settings = login_settings.clone();
            let selected_service = selected_service.clone();

            Box::new(move || match password.get().is_empty() {
                _ if username.get().is_empty() => vec![ClickAction::FocusNext(FocusMode::FocusPrevious)],
                true => Vec::new(),
                false => {
                    // TODO: Deduplicate code
                    let service_id = selected_service.cloned();

                    login_settings.mutate(|login_settings| {
                        login_settings.recent_service_id = Some(service_id);

                        let saved_settings = login_settings.service_settings.entry(service_id).or_default();
                        saved_settings.username = username.cloned();
                        saved_settings.password = password.cloned();
                    });

                    vec![ClickAction::Custom(UserEvent::LogIn {
                        service_id: selected_service.cloned(),
                        username: username.cloned(),
                        password: password.cloned(),
                    })]
                }
            })
        };

        let remember_username = {
            let service_id = selected_service.clone();

            login_settings.mapped(move |login_settings| &login_settings.service_settings.get(&service_id.get()).unwrap().remember_username)
        };

        let remember_password = {
            let service_id = selected_service.clone();

            login_settings.mapped(move |login_settings| &login_settings.service_settings.get(&service_id.get()).unwrap().remember_password)
        };

        let elements = vec![
            Text::default().with_text("Select service").wrap(),
            PickList::default()
                .with_options(options)
                .with_selected(selected_service)
                .with_event(service_changed)
                .wrap(),
            Text::default().with_text("Account data").wrap(),
            InputFieldBuilder::new()
                .with_state(username)
                .with_ghost_text("Username")
                .with_enter_action(username_action)
                .with_length(24)
                .build()
                .wrap(),
            InputFieldBuilder::new()
                .with_state(password)
                .with_ghost_text("Password")
                .with_enter_action(password_action)
                .with_length(24)
                .hidden()
                .build()
                .wrap(),
            Container::new({
                vec![
                    StateButtonBuilder::new()
                        .with_text("Remember username")
                        .with_remote(remember_username.new_remote())
                        .with_event(remember_username.toggle_action())
                        .with_transparent_background()
                        .build()
                        .wrap(),
                    StateButtonBuilder::new()
                        .with_text("Remember password")
                        .with_remote(remember_password.new_remote())
                        .with_event(remember_password.toggle_action())
                        .with_transparent_background()
                        .build()
                        .wrap(),
                ]
            })
            .wrap(),
            ButtonBuilder::new()
                .with_text("Log in")
                .with_disabled_selector(selector)
                .with_event(Box::new(login_action))
                .build()
                .wrap(),
        ];

        WindowBuilder::new()
            .with_title("Log In".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size_bound(size_bound!(200 > 300 < 400, ?))
            .with_elements(elements)
            .with_theme_kind(InterfaceThemeKind::Menu)
            .build(window_cache, application, available_space)
    }
}
