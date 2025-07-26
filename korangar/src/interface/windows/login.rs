use std::ops::Not;

use derive_new::new;
use korangar_interface::components::drop_down::DefaultClickHandler;
use korangar_interface::components::text_box::DefaultHandler;
use korangar_interface::element::id::FocusIdExt;
use korangar_interface::event::{ClickAction, Event, EventQueue, Toggle};
use korangar_interface::window::{CustomWindow, StateWindow, Window, WindowTrait};
use rust_state::{Context, ManuallyAssertExt, MapLookupExt, Path, Selector};

use crate::graphics::Color;
use crate::input::UserEvent;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::{WindowCache, WindowClass};
use crate::loaders::{ClientInfo, ClientInfoPathExt};
use crate::settings::{LoginSettings, LoginSettingsPathExt, ServiceSettings, ServiceSettingsPathExt};
use crate::state::{ClientState, ClientThemeType, LoginWindowState, LoginWindowStatePathExt};

const MAXIMUM_USERNAME_LENGTH: usize = 24;
const MAXIMUM_PASSWORD_LENGTH: usize = 24;

struct SelectedServicePath<P, S> {
    window_state_path: P,
    service_settings_path: S,
}

impl<P, S> SelectedServicePath<P, S>
where
    P: Path<ClientState, LoginWindowState>,
    S: Path<ClientState, LoginSettings>,
{
    fn new(window_state_path: P, service_settings_path: S) -> Self {
        Self {
            window_state_path,
            service_settings_path,
        }
    }
}

impl<P, S> Clone for SelectedServicePath<P, S>
where
    P: Path<ClientState, LoginWindowState>,
    S: Path<ClientState, LoginSettings>,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<P, S> Copy for SelectedServicePath<P, S>
where
    P: Path<ClientState, LoginWindowState>,
    S: Path<ClientState, LoginSettings>,
{
}

impl<P, S> Selector<ClientState, ServiceSettings> for SelectedServicePath<P, S>
where
    P: Path<ClientState, LoginWindowState>,
    S: Path<ClientState, LoginSettings>,
{
    fn select<'a>(&'a self, state: &'a ClientState) -> Option<&'a ServiceSettings> {
        self.follow(state)
    }
}

impl<P, S> Path<ClientState, ServiceSettings> for SelectedServicePath<P, S>
where
    P: Path<ClientState, LoginWindowState>,
    S: Path<ClientState, LoginSettings>,
{
    fn follow<'a>(&self, state: &'a ClientState) -> Option<&'a ServiceSettings> {
        let selected_service_path = self.window_state_path.selected_service();
        // SAFETY:
        //
        // Unwrapping here is safe because of the bounds.
        let selected_service = selected_service_path.follow(state).unwrap();

        // SAFETY:
        //
        // First unwrap here is safe because of the bounds.
        // Second unwrap guaranteed to be safe by the ClientState. When it loads, it
        // makes sure each available service has a settings entry.
        self.service_settings_path
            .follow(state)
            .unwrap()
            .service_settings
            .get(selected_service)
    }

    fn follow_mut<'a>(&self, state: &'a mut ClientState) -> Option<&'a mut ServiceSettings> {
        let selected_service_path = self.window_state_path.selected_service();
        // SAFETY:
        //
        // Unwrapping here is safe because of the bounds.
        let selected_service = *selected_service_path.follow_mut(state).unwrap();

        // SAFETY:
        //
        // First unwrap here is safe because of the bounds.
        // Second unwrap guaranteed to be safe by the ClientState. When it loads, it
        // makes sure each available service has a settings entry.
        self.service_settings_path
            .follow_mut(state)
            .unwrap()
            .service_settings
            .get_mut(&selected_service)
    }
}

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

    fn to_window<'a>(self) -> impl WindowTrait<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let selected_service_path = SelectedServicePath::new(self.window_state_path, self.service_settings_path);
        let username_path = selected_service_path.username();
        let password_path = selected_service_path.password();
        let remember_username_path = selected_service_path.remember_username();
        let remember_password_path = selected_service_path.remember_password();

        let disabled = move |state: &Context<ClientState>| !state.get(&username_path).is_empty() && !state.get(&password_path).is_empty();

        let login_action = move |state: &Context<ClientState>, queue: &mut EventQueue<ClientState>| {
            let selected_service_path = self.window_state_path.selected_service();
            let selected_service_id = state.get(&selected_service_path);
            let username = state.get(&username_path);
            let password = state.get(&password_path);

            // Remember which service was selected so we can select it next time the client
            // starts.
            state.update_value(self.service_settings_path.recent_service_id(), Some(*selected_service_id));

            queue.queue(UserEvent::LogIn {
                service_id: *selected_service_id,
                username: username.clone(),
                password: password.clone(),
            });
        };

        struct UsernameTextBox;
        struct PasswordTextBox;

        let username_action = move |state: &Context<ClientState>, queue: &mut EventQueue<ClientState>| {
            if !state.get(&username_path).is_empty() {
                queue.queue(Event::FocusElement {
                    focus_id: PasswordTextBox.focus_id(),
                });
            }
        };

        let password_action = move |state: &Context<ClientState>, queue: &mut EventQueue<ClientState>| {
            let username = state.get(&username_path);

            if username.is_empty() {
                queue.queue(Event::FocusElement {
                    focus_id: UsernameTextBox.focus_id(),
                });
                return;
            }

            login_action(state, queue);
        };

        window! {
            title: "Log In",
            class: Self::window_class(),
            theme: ClientThemeType::Menu,
            minimum_width: 450.0,
            maximum_width: 450.0,
            elements: (
                text! { text: "Select service" },
                drop_down! {
                    options: self.client_info_path.services(),
                    selected: self.window_state_path.selected_service(),
                    click_handler: DefaultClickHandler::new(self.window_state_path.selected_service(), self.client_info_path.services()),
                },
                text! { text: "Account data" },
                fragment! {
                    gaps: 5.0,
                    children: (
                        text_box! {
                            text: "Username",
                            state: username_path,
                            input_handler: DefaultHandler::<_, _, MAXIMUM_USERNAME_LENGTH>::new(username_path, username_action),
                            focus_id: UsernameTextBox,
                        },
                        text_box! {
                            text: "Password",
                            state: password_path,
                            input_handler: DefaultHandler::<_, _, MAXIMUM_PASSWORD_LENGTH>::new(password_path, password_action),
                            focus_id: PasswordTextBox,
                            hidable: true,
                        },
                        state_button! {
                            text: "Remember username",
                            state: remember_username_path,
                            event: Toggle(remember_username_path),
                            background_color: Color::TRANSPARENT,
                            hovered_background_color: Color::TRANSPARENT,
                            text_alignment: HorizontalAlignment::Left { offset: 50.0 },
                        },
                        state_button! {
                            text: "Remember password",
                            state: remember_password_path,
                            event: Toggle(remember_password_path),
                            background_color: Color::TRANSPARENT,
                            hovered_background_color: Color::TRANSPARENT,
                            text_alignment: HorizontalAlignment::Left { offset: 50.0 },
                        },
                    ),
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
