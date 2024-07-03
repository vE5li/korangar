use derive_new::new;
use korangar_interface::elements::{ElementWrap, Text};
use korangar_interface::size_bound;
use korangar_interface::windows::{PrototypeWindow, Window, WindowBuilder};
use rust_state::Context;

use crate::graphics::Color;
use crate::interface::layout::ScreenSize;
use crate::interface::theme::InterfaceThemeKind;
use crate::interface::windows::WindowCache;
use crate::GameState;

#[derive(new)]
pub struct ErrorWindow {
    message: String,
}

impl PrototypeWindow<GameState> for ErrorWindow {
    fn to_window(&self, window_cache: &WindowCache, application: &Context<GameState>, available_space: ScreenSize) -> Window<GameState> {
        let elements = vec![
            Text::default()
                .with_text(self.message.clone())
                .with_foreground_color(|_, _| Color::rgb_u8(220, 100, 100))
                .wrap(),
        ];

        WindowBuilder::new()
            .with_title("Error".to_string())
            .with_size_bound(size_bound!(300 > 400 < 500, ?))
            .with_elements(elements)
            .closable()
            .with_theme_kind(InterfaceThemeKind::Menu)
            .build(window_cache, application, available_space)
    }
}
