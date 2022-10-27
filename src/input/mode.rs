use crate::interface::{ElementCell, ItemSource};
use crate::inventory::Item;

#[derive(Default)]
pub enum MouseInputMode {
    MoveItem(ItemSource, Item),
    MoveSkill(usize),
    MoveInterface(usize),
    ResizeInterface(usize),
    DragElement((ElementCell, usize)),
    ClickInterface,
    RotateCamera,
    #[default]
    None,
}

impl MouseInputMode {
    pub fn is_none(&self) -> bool {
        matches!(self, MouseInputMode::None)
    }
}
