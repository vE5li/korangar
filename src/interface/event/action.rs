use super::{ItemSource, SkillSource};
use crate::input::UserEvent;
use crate::interface::{ChangeEvent, ElementCell, FocusMode, Position, PrototypeWindow, Size, Tracker};
use crate::inventory::{Item, Skill};

pub enum ClickAction {
    FocusElement,
    FocusNext(FocusMode),
    ChangeEvent(ChangeEvent),
    Event(UserEvent),
    DragElement,
    MoveItem(ItemSource, Item),
    MoveSkill(SkillSource, Skill),
    MoveInterface,
    OpenWindow(Box<dyn PrototypeWindow>),
    CloseWindow,
    OpenPopup {
        element: ElementCell,
        position_tracker: Tracker<Position>,
        size_tracker: Tracker<Size>,
    },
    ClosePopup,
}
