use derive_new::new;
use procedural::size_bound;

use crate::graphics::Color;
use crate::interface::*;

#[derive(new)]
pub struct ErrorWindow {
    message: String,
}

impl PrototypeWindow for ErrorWindow {
    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, available_space: ScreenSize) -> Window {
        let elements = vec![
            Text::default()
                .with_text(self.message.clone())
                .with_foreground_color(|_| Color::rgb_u8(220, 100, 100))
                .wrap(),
        ];

        WindowBuilder::new()
            .with_title("Error".to_string())
            .with_size_bound(size_bound!(300 > 400 < 500, ?))
            .with_elements(elements)
            .closable()
            .with_theme_kind(ThemeKind::Menu)
            .build(window_cache, interface_settings, available_space)
    }
}
