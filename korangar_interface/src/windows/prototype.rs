use crate::application::Application;
use crate::Window;

pub trait PrototypeWindow<App>
where
    App: Application,
{
    fn window_class(&self) -> Option<&str> {
        None
    }

    fn to_window(&self, window_cache: &App::Cache, application: &App, available_space: App::Size) -> Window<App>;
}
