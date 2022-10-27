use super::ItemSource;
use crate::input::UserEvent;
use crate::interface::{ChangeEvent, FocusMode, PrototypeWindow};
use crate::inventory::Item;

pub enum ClickAction {
    FocusElement,
    FocusNext(FocusMode),
    ChangeEvent(ChangeEvent),
    Event(UserEvent),
    DragElement,
    MoveItem(ItemSource, Item),
    MoveInterface,
    OpenWindow(Box<dyn PrototypeWindow>),
    CloseWindow,
}
