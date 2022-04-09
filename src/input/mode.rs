pub enum MouseInputMode {
    MoveInterface(usize),
    ResizeInterface(usize),
    ClickInterface,
    None,
}

impl MouseInputMode {

    pub fn is_none(&self) -> bool {
        return matches!(self, MouseInputMode::None);
    }
}
