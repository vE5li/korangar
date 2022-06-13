use interface::types::{ WindowCache, Size, InterfaceSettings };
use interface::traits::Window;

pub trait PrototypeWindow {

    fn window_class(&self) -> Option<&str> {
        None
    }

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, avalible_space: Size) -> Box<dyn Window + 'static>;
}
