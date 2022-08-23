use crate::interface::{ WindowCache, Size, InterfaceSettings };
use crate::interface::Window;

pub trait PrototypeWindow {

    fn window_class(&self) -> Option<&str> {
        None
    }

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, avalible_space: Size) -> Box<dyn Window + 'static>;
}
