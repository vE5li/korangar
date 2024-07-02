use super::ChangeEvent;
use crate::_Tracker;
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
        position_tracker: _Tracker<App::Position>,
        size_tracker: _Tracker<App::Size>,
    },
    ClosePopup,
    Custom(App::CustomEvent),
}
