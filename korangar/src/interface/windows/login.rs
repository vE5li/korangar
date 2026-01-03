use korangar_interface::components::text_box::DefaultHandler;
use korangar_interface::element::StateElement;
use korangar_interface::element::id::FocusIdExt;
use korangar_interface::window::{CustomWindow, Window};
use rust_state::{Context, Path, RustState, Selector};

use crate::graphics::{Color, ShadowPadding};
use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::{ClientInfo, ClientInfoPathExt, OverflowBehavior, ServiceId};
use crate::settings::{LoginSettings, LoginSettingsPathExt, ServiceSettings, ServiceSettingsPathExt};
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};

const MAXIMUM_USERNAME_LENGTH: usize = 24;
const MAXIMUM_PASSWORD_LENGTH: usize = 24;

// TODO: Maybe move this somewhere else
pub struct SelectedServicePath<P, S> {
    window_state_path: P,
    service_settings_path: S,
}

impl<P, S> SelectedServicePath<P, S>
where
    P: Path<ClientState, LoginWindowState>,
    S: Path<ClientState, LoginSettings>,
{
    pub fn new(window_state_path: P, service_settings_path: S) -> Self {
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

/// Internal state of the login window.
#[derive(RustState, StateElement)]
pub struct LoginWindowState {
    selected_service: ServiceId,
}

impl LoginWindowState {
    pub fn new(selected_service: ServiceId) -> Self {
        Self { selected_service }
    }
}

pub struct LoginWindow<A, B, C> {
    window_state_path: A,
    service_settings_path: B,
    client_info_path: C,
}

impl<A, B, C> LoginWindow<A, B, C> {
    pub fn new(window_state_path: A, service_settings_path: B, client_info_path: C) -> Self {
        Self {
            window_state_path,
            service_settings_path,
            client_info_path,
        }
    }
}

impl<A, B, C> CustomWindow<ClientState> for LoginWindow<A, B, C>
where
    A: Path<ClientState, LoginWindowState>,
    B: Path<ClientState, LoginSettings>,
    C: Path<ClientState, ClientInfo>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Login)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let selected_service_path = SelectedServicePath::new(self.window_state_path, self.service_settings_path);
        let username_path = selected_service_path.username();
        let password_path = selected_service_path.password();
        let remember_username_path = selected_service_path.remember_username();
        let remember_password_path = selected_service_path.remember_password();

        let disabled_selector = ComputedSelector::new_default(move |state: &ClientState| {
            selected_service_path.username().follow(state).unwrap().is_empty()
                || selected_service_path.password().follow(state).unwrap().is_empty()
        });

        let login_action = move |state: &Context<ClientState>, queue: &mut EventQueue<ClientState>| {
            let selected_service_path = self.window_state_path.selected_service();
            let selected_service_id = state.get(&selected_service_path);
            let username = state.get(&username_path);
            let password = state.get(&password_path);

            // Remember which service was selected so we can select it next time the client
            // starts.
            state.update_value(self.service_settings_path.recent_service_id(), Some(*selected_service_id));

            queue.queue(InputEvent::LogIn {
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
            title: client_state().localization().log_in_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::Menu,
            minimum_width: 450.0,
            maximum_width: 450.0,
            elements: (
                text! { text: client_state().localization().select_service_text() },
                drop_down! {
                    options: self.client_info_path.services(),
                    selected: self.window_state_path.selected_service(),
                    click_handler: DefaultClickHandler::new(self.window_state_path.selected_service(), self.client_info_path.services()),
                },
                text! { text: client_state().localization().account_data_text() },
                fragment! {
                    // TODO: Theme this
                    gaps: 8.0,
                    children: (
                        text_box! {
                            ghost_text: client_state().localization().username_text(),
                            state: username_path,
                            input_handler: DefaultHandler::<_, _, MAXIMUM_USERNAME_LENGTH>::new(username_path, username_action),
                            focus_id: UsernameTextBox,
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        text_box! {
                            ghost_text: client_state().localization().password_text(),
                            state: password_path,
                            input_handler: DefaultHandler::<_, _, MAXIMUM_PASSWORD_LENGTH>::new(password_path, password_action),
                            focus_id: PasswordTextBox,
                            hidable: true,
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                    ),
                },
                fragment! {
                    // TODO: Theme this
                    gaps: 8.0,
                    children: (
                        state_button! {
                            text: client_state().localization().remember_username_text(),
                            state: remember_username_path,
                            event: Toggle(remember_username_path),
                            background_color: Color::TRANSPARENT,
                            hovered_background_color: Color::TRANSPARENT,
                            shadow_padding: ShadowPadding::default(),
                        },
                        state_button! {
                            text: client_state().localization().remember_password_text(),
                            state: remember_password_path,
                            event: Toggle(remember_password_path),
                            background_color: Color::TRANSPARENT,
                            hovered_background_color: Color::TRANSPARENT,
                            shadow_padding: ShadowPadding::default(),
                        },
                    ),
                },
                button! {
                    text: client_state().localization().log_in_button_text(),
                    disabled: disabled_selector,
                    disabled_tooltip: client_state().localization().log_in_button_tooltip(),
                    event: login_action,
                },
            ),
        }
    }
}
