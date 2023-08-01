use super::{ItemSource, SkillSource};
use crate::input::UserEvent;
use crate::interface::{ChangeEvent, FocusMode, PrototypeWindow};
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
}
