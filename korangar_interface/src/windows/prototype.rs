pub use interface_procedural::PrototypeWindow;
use rust_state::Context;

use crate::application::Application;
use crate::Window;

pub trait PrototypeWindow<App>
where
    App: Application,
{
    fn window_class(&self) -> Option<&str> {
        None
    }

    fn to_window(&self, window_cache: &App::Cache, application: &Context<App>, available_space: App::Size) -> Window<App>;
}
