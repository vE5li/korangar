use derive_new::new;
use korangar_interface::window::{CustomWindow, WindowTrait};

use crate::graphics::Color;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;

#[derive(new)]
pub struct ErrorWindow {
    message: String,
}

impl CustomWindow<ClientState> for ErrorWindow {
    fn to_window<'a>(self) -> impl WindowTrait<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Error",
            theme: InterfaceThemeType::Menu,
            closable: true,
            elements: (text! {
                text: self.message,
                color: Color::rgb_u8(220, 100, 100)
            },),
        }
    }
}
