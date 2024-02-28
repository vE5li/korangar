use crate::interface::{InterfaceSettings, ScreenSize, Window, WindowCache};

pub trait PrototypeWindow {
    fn window_class(&self) -> Option<&str> {
        None
    }

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, available_space: ScreenSize) -> Window;
}
