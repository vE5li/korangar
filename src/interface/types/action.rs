use input::UserEvent;
use interface::PrototypeWindow;

pub enum ClickAction {
    Event(UserEvent),
    DragElement,
    MoveInterface,
    OpenWindow(Box<dyn PrototypeWindow>),
    CloseWindow,
}
