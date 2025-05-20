use std::ops::Not;

use derive_new::new;
use korangar_interface::components::text_box::DefaultHandler;
use korangar_interface::event::{ClickAction, Event, EventQueue, Toggle};
use korangar_interface::window::{CustomWindow, PrototypeWindow, Window, WindowTrait};
use rust_state::{Context, ManuallyAssertExt, MapLookupExt, Path};

use crate::graphics::Color;
use crate::input::UserEvent;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::{WindowCache, WindowClass};
use crate::loaders::{ClientInfo, ClientInfoPathExt};
use crate::settings::{LoginSettings, LoginSettingsPathExt, ServiceSettings, ServiceSettingsPathExt};
use crate::state::{ClientState, ClientThemeType, LoginWindowState, LoginWindowStatePathExt};

pub struct LoginWindow<P, S, C> {
    window_state_path: P,
    service_settings_path: S,
    client_info_path: C,
}

impl<P, S, C> LoginWindow<P, S, C> {
    pub fn new(window_state_path: P, service_settings_path: S, client_info_path: C) -> Self {
        Self {
            window_state_path,
            service_settings_path,
            client_info_path,
        }
    }
}

impl<P, S, C> CustomWindow<ClientState> for LoginWindow<P, S, C>
where
    P: Path<ClientState, LoginWindowState>,
    S: Path<ClientState, LoginSettings>,
    C: Path<ClientState, ClientInfo>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Login)
    }

    fn to_window<'a>(
        self,
        state: &Context<ClientState>,
        window_cache: &WindowCache,
        available_space: ScreenSize,
    ) -> impl WindowTrait<ClientState> + 'a {
        use korangar_interface::prelude::*;

        // TODO: Move this to the main function where the ClientState is created.
        let mut login_settings = LoginSettings::new();

        // let options = state
        //     .get(&self.client_info_path.services())
        //     .iter()
        //     .map(|service| (service.display_name.clone().unwrap(),
        // service.service_id()))     .collect();

        let disabled = move |state: &Context<ClientState>| {
            !state.get(&self.window_state_path.username()).is_empty() && !state.get(&self.window_state_path.password()).is_empty()
        };

        let login_action = move |state: &Context<ClientState>, queue: &mut EventQueue<ClientState>| {
            let username_path = self.window_state_path.username();
            let username = state.get(&username_path);

            let passwor_path = self.window_state_path.password();
            let password = state.get(&passwor_path);

            let selected_service_path = self.window_state_path.selected_service();
            let selected_service = state.get(&selected_service_path);

            state.map_insert_default(self.service_settings_path.service_settings(), *selected_service);
            state.update_value(
                self.service_settings_path
                    .service_settings()
                    .lookup(*selected_service)
                    .username()
                    .manually_asserted(),
                username.clone(),
            );
            state.update_value(
                self.service_settings_path
                    .service_settings()
                    .lookup(*selected_service)
                    .password()
                    .manually_asserted(),
                password.clone(),
            );

            queue.queue(UserEvent::LogIn {
                service_id: *selected_service,
                username: username.clone(),
                password: password.clone(),
            });
        };

        let username_action = move |state: &Context<ClientState>, queue: &mut EventQueue<ClientState>| {
            if !state.get(&self.window_state_path.username()).is_empty() {
                queue.queue(Event::FocusNext);
            }
        };

        let password_action = move |state: &Context<ClientState>, queue: &mut EventQueue<ClientState>| {
            let username_path = self.window_state_path.username();
            let username = state.get(&username_path);

            if username.is_empty() {
                queue.queue(Event::FocusPrevious);
                return;
            }

            let passwor_path = self.window_state_path.password();
            let password = state.get(&passwor_path);

            let selected_service_path = self.window_state_path.selected_service();
            let selected_service = state.get(&selected_service_path);

            state.map_insert_default(self.service_settings_path.service_settings(), *selected_service);
            state.update_value(
                self.service_settings_path
                    .service_settings()
                    .lookup(*selected_service)
                    .username()
                    .manually_asserted(),
                username.clone(),
            );
            state.update_value(
                self.service_settings_path
                    .service_settings()
                    .lookup(*selected_service)
                    .password()
                    .manually_asserted(),
                password.clone(),
            );

            queue.queue(UserEvent::LogIn {
                service_id: *selected_service,
                username: username.clone(),
                password: password.clone(),
            });
        };

        window! {
            title: "Log In",
            class: Some(WindowClass::Login),
            theme: ClientThemeType::Menu,
            elements: (
                text! { text: "Select service" },
                // pick_list! { options: options, selected: selected_service },
                text! { text: "Account data" },
                text_box! {
                    text: "Username",
                    state: self.window_state_path.username(),
                    input_handler: DefaultHandler(self.window_state_path.username()),
                    // event: username_action,
                    // length: 24,
                },
                text_box! {
                    text: "Password",
                    state: self.window_state_path.password(),
                    input_handler: DefaultHandler(self.window_state_path.password()),
                    // event: password_action,
                    // length: 24,
                    // hidden: true,
                },
                state_button! {
                    text: "Remember username",
                    state: self.window_state_path.remember_username(),
                    event: Toggle(self.window_state_path.remember_username()),
                    background_color: Color::TRANSPARENT,
                },
                state_button! {
                    text: "Remember password",
                    state: self.window_state_path.remember_password(),
                    event: Toggle(self.window_state_path.remember_password()),
                    background_color: Color::TRANSPARENT,
                },
                button! {
                    text: "Log in",
                    // disabled: selector,
                    event: login_action,
                },
            ),
        }
    }
}
