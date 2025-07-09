use rust_state::{Context, Path};

use super::EventQueue;
use crate::application::Application;

// pub enum ClickAction<App>
// where
//     App: Application,
// {
//     FocusElement,
//     FocusNext(FocusMode),
//     ChangeEvent(ChangeEvent),
//     DragElement,
//     Move(App::DropResource),
//     MoveInterface,
//     OpenWindow(Box<dyn StateWindow<App>>),
//     CloseWindow,
//     OpenPopup {
//         element: ElementCell<App>,
//         position_tracker: Tracker<App::Position>,
//         size_tracker: Tracker<App::Size>,
//     },
//     ClosePopup,
//     Custom(App::CustomEvent),
// }

pub trait ClickAction<App: Application> {
    fn execute(&self, state: &Context<App>, queue: &mut EventQueue<App>);
}

impl<App, F> ClickAction<App> for F
where
    App: Application,
    F: Fn(&Context<App>, &mut EventQueue<App>),
{
    fn execute(&self, state: &Context<App>, queue: &mut EventQueue<App>) {
        self(state, queue)
    }
}

pub struct Toggle<T>(pub T);

impl<T, App> ClickAction<App> for Toggle<T>
where
    App: Application,
    T: Path<App, bool>,
{
    fn execute(&self, state: &Context<App>, _: &mut EventQueue<App>) {
        state.update_value_with(self.0, |value| {
            *value = !*value;
        });
    }
}
