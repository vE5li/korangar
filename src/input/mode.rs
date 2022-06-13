use interface::types::ElementCell;

pub enum MouseInputMode {
    MoveInterface(usize),
    ResizeInterface(usize),
    DragElement((ElementCell, usize)),
    ClickInterface,
    None,
}

impl MouseInputMode {

    pub fn is_none(&self) -> bool {
        return matches!(self, MouseInputMode::None);
    }
}
