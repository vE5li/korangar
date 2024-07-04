use std::ops::Not;

use derive_new::new;
use korangar_interface::elements::{
    ButtonBuilder, Container, ElementWrap, FocusMode, InputFieldBuilder, PickList, StateButtonBuilder, Text,
};
use korangar_interface::event::ClickAction;
use korangar_interface::size_bound;
use korangar_interface::state::{PlainTrackedState, TrackedState, TrackedStateBinary, TrackedStateClone, TrackedStateExt};
use korangar_interface::windows::{PrototypeWindow, Window, WindowBuilder};
use rust_state::{Context, MapLookup, SafeUnwrap, Selector, Tracker};

use crate::input::UserEvent;
use crate::interface::layout::ScreenSize;
use crate::interface::theme::InterfaceThemeKind;
use crate::interface::windows::WindowCache;
use crate::loaders::client::{LoginSettings, ServiceSettings};
use crate::loaders::{ClientInfo, ServiceId};
use crate::{GameState, GameStateLoginSettingsPath};

#[derive(new)]
pub struct LoginWindow<'a> {
    client_info: &'a ClientInfo,
}

impl<'a> LoginWindow<'a> {
    pub const WINDOW_CLASS: &'static str = "login";
}

impl<'a> PrototypeWindow<GameState> for LoginWindow<'a> {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(&self, window_cache: &WindowCache, application: &Context<GameState>, available_space: ScreenSize) -> Window<GameState> {
        // let mut login_settings = LoginSettings::new();

        let options = self
            .client_info
            .services
            .iter()
            .map(|service| (service.display_name.clone().unwrap(), service.service_id()))
            .collect();

        // let login_settings =
        // application.get_safe(&GameStateLoginSettingsPath::default());

        // FIX: This will panic when no services are present. What is the correct
        // behavior?
        /*let selected_service = login_settings
        .recent_service_id
        // Only use the recent server if it is still in the client info
        .filter(|&recent_service_id| {
            self.client_info
                .services
                .iter()
                .any(|service| service.service_id() == recent_service_id)
        })
        .unwrap_or_else(|| self.client_info.services[0].service_id());*/

        // let saved_settings =
        // login_settings.service_settings.entry(selected_service).or_default();

        // let username = PlainTrackedState::new(saved_settings.username.clone());
        // let password = PlainTrackedState::new(saved_settings.password.clone());
        //
        // let selected_service = PlainTrackedState::new(selected_service);
        // let login_settings = PlainTrackedState::new(login_settings);

        let selector = |state: &Tracker<GameState>| {
            !state.get_safe(&GameState::username()).is_empty() && !state.get_safe(&GameState::password()).is_empty()
        };

        let service_changed = Box::new(move |state: &Context<GameState>| {
            // let service_id = *state.get_safe(&service_selector);
            // state.update_value(&LoginSettings::service_settings(login_settings_selector),
            // Some(service_id));

            // let saved_settings =
            //     login_settings.mutate(|login_settings|
            // login_settings.service_settings.entry(service_id).or_default().clone());

            let saved_username = String::new();
            let saved_password = String::new();

            state.update_value(&GameState::username(), saved_username);
            state.update_value(&GameState::password(), saved_password);

            Vec::new()
        });

        let login_action = move |state: &Context<GameState>| {
            // TODO: Deduplicate code
            let username = state.get_safe(&GameState::username()).clone();
            let password = state.get_safe(&GameState::password()).clone();
            let service_id = *state.get_safe(&GameState::selected_service());

            state.update_value(&LoginSettings::recent_service_id(GameState::login_settings()), Some(service_id));

            // login_settings.mutate(|login_settings| {
            // login_settings.recent_service_id = Some(service_id);

            // let saved_settings =
            // login_settings.service_settings.entry(service_id).or_default();
            // saved_settings.username = username.cloned();
            // saved_settings.password = password.cloned();
            // });

            vec![ClickAction::Custom(UserEvent::LogIn {
                service_id,
                username,
                password,
            })]
        };

        let username_action = {
            Box::new(move |state: &Context<GameState>| {
                state
                    .get_safe(&GameState::username())
                    .is_empty()
                    .not()
                    .then_some(vec![ClickAction::FocusNext(FocusMode::FocusNext)])
                    .unwrap_or_default()
            })
        };

        let password_action = Box::new(move |state: &Context<GameState>| {
            let username = state.get_safe(&GameState::username());
            let password = state.get_safe(&GameState::password());

            match password.is_empty() {
                _ if username.is_empty() => vec![ClickAction::FocusNext(FocusMode::FocusPrevious)],
                true => Vec::new(),
                false => {
                    // TODO: Deduplicate code
                    let service_id = *state.get_safe(&GameState::selected_service());

                    state.update_value(&LoginSettings::recent_service_id(GameState::login_settings()), Some(service_id));

                    /* login_settings.mutate(|login_settings| {
                        // login_settings.recent_service_id = Some(service_id);

                        let saved_settings = login_settings.service_settings.entry(service_id).or_default();
                        saved_settings.username = username.cloned();
                        saved_settings.password = password.cloned();
                    }); */

                    vec![ClickAction::Custom(UserEvent::LogIn {
                        service_id,
                        username: username.clone(),
                        password: password.clone(),
                    })]
                }
            }
        });

        let service_id = *application.get_safe(&GameState::selected_service());
        let remember_username = ServiceSettings::remember_username(MapLookup::new(
            LoginSettings::service_settings(GameState::login_settings()),
            service_id,
        ));
        let remember_password = ServiceSettings::remember_password(MapLookup::new(
            LoginSettings::service_settings(GameState::login_settings()),
            service_id,
        ));

        let elements = vec![
            Text::default().with_text("Select service").wrap(),
            PickList::default()
                .with_options(options)
                .with_selected(GameState::selected_service())
                .with_event(service_changed)
                .wrap(),
            Text::default().with_text("Account data").wrap(),
            InputFieldBuilder::new()
                .with_state(GameState::username())
                .with_ghost_text("Username")
                .with_enter_action(username_action)
                .with_length(24)
                .build()
                .wrap(),
            InputFieldBuilder::new()
                .with_state(GameState::password())
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
                        .with_remote(remember_username)
                        .with_toggle_event()
                        .with_transparent_background()
                        .build()
                        .wrap(),
                    StateButtonBuilder::new()
                        .with_text("Remember password")
                        .with_remote(remember_password)
                        .with_toggle_event()
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
