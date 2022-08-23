use crate::input::UserEvent;
use crate::interface::PrototypeWindow;

pub enum ClickAction {
    FocusElement,
    Event(UserEvent),
    DragElement,
    MoveInterface,
    OpenWindow(Box<dyn PrototypeWindow>),
    CloseWindow,
}
