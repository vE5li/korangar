use crate::interface::types::ElementCell;

pub enum MouseInputMode {
    MoveInterface(usize),
    ResizeInterface(usize),
    DragElement((ElementCell, usize)),
    ClickInterface,
    None,
}

impl MouseInputMode {

    pub fn is_none(&self) -> bool {
        matches!(self, MouseInputMode::None)
    }
}
