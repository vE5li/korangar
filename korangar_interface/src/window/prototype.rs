use rust_state::{Context, Path};

use super::WindowTrait;
use crate::application::Appli;

// TODO: Rename this to StateWindow
pub trait PrototypeWindow<App>
where
    App: Appli,
{
    fn window_class() -> Option<App::WindowClass> {
        None
    }

    fn to_window<'a>(self_path: impl Path<App, Self>) -> impl WindowTrait<App> + 'a;

    // TODO: Add `to_window_mut`
}

pub trait CustomWindow<App>
where
    App: Appli,
{
    fn window_class() -> Option<App::WindowClass> {
        None
    }

    fn to_window<'a>(self) -> impl WindowTrait<App> + 'a;
}
