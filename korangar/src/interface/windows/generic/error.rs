use derive_new::new;
use korangar_interface::window::{CustomWindow, PrototypeWindow, Window, WindowTrait};
use rust_state::Context;

use crate::graphics::Color;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::WindowCache;
use crate::state::{ClientState, ClientThemeType};

#[derive(new)]
pub struct ErrorWindow {
    message: String,
}

impl CustomWindow<ClientState> for ErrorWindow {
    fn to_window<'a>(
        self,
        state: &Context<ClientState>,
        window_cache: &WindowCache,
        available_space: ScreenSize,
    ) -> impl WindowTrait<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Error",
            theme: ClientThemeType::Menu,
            closable: true,
            elements: (text! {
                text: self.message,
                color: Color::rgb_u8(220, 100, 100)
            },),
        }
    }
}
