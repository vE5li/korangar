use super::ChangeEvent;
use crate::Tracker;
use crate::application::Application;
use crate::elements::{ElementCell, FocusMode};
use crate::windows::PrototypeWindow;

pub enum ClickAction<App>
where
    App: Application,
{
    FocusElement,
    FocusNext(FocusMode),
    ChangeEvent(ChangeEvent),
    DragElement,
    Move(App::DropResource),
    MoveInterface,
    OpenWindow(Box<dyn PrototypeWindow<App>>),
    CloseWindow,
    OpenPopup {
        element: ElementCell<App>,
        position_tracker: Tracker<App::Position>,
        size_tracker: Tracker<App::Size>,
    },
    ClosePopup,
    Custom(App::CustomEvent),
}
