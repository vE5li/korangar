use crate::interface::ElementCell;

pub enum MouseInputMode {
    MoveInterface(usize),
    ResizeInterface(usize),
    DragElement((ElementCell, usize)),
    ClickInterface,
    RotateCamera,
    None,
}

impl MouseInputMode {

    pub fn is_none(&self) -> bool {
        matches!(self, MouseInputMode::None)
    }
}
