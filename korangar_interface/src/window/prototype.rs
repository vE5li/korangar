use rust_state::{Context, Path};

use super::WindowTrait;
use crate::application::Appli;

// TODO: Rename this to StateWindow
pub trait PrototypeWindow<App>
where
    App: Appli,
{
    fn window_class() -> Option<&'static str> {
        None
    }

    fn to_window<'a>(
        self_path: impl Path<App, Self>,
        window_cache: &App::Cache,
        application: &App,
        available_space: App::Size,
    ) -> impl WindowTrait<App> + 'a;

    // TODO: Add `to_window_mut`
}

pub trait CustomWindow<App>
where
    App: Appli,
{
    fn window_class() -> Option<&'static str> {
        None
    }

    fn to_window<'a>(self, state: &Context<App>, window_cache: &App::Cache, available_space: App::Size) -> impl WindowTrait<App> + 'a;
}
