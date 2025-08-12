use derive_new::new;
use korangar_interface::window::{CustomWindow, Window};

use crate::graphics::Color;
use crate::state::theme::InterfaceThemeType;
use crate::state::translation::TranslationPathExt;
use crate::state::{ClientState, ClientStatePathExt, client_state};

#[derive(new)]
pub struct ErrorWindow {
    message: String,
}

impl CustomWindow<ClientState> for ErrorWindow {
    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: client_state().translation().error_window_title(),
            theme: InterfaceThemeType::Menu,
            closable: true,
            elements: (
                text! {
                    text: self.message,
                    color: Color::rgb_u8(220, 100, 100)
                },
            ),
        }
    }
}
