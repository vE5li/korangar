use crate::input::UserEvent;
use crate::interface::PrototypeWindow;

pub enum ClickAction {
    Event(UserEvent),
    DragElement,
    MoveInterface,
    OpenWindow(Box<dyn PrototypeWindow>),
    CloseWindow,
}
