pub enum MouseInputMode {
    MoveInterface(usize),
    Click,
    None,
}

impl MouseInputMode {

    pub fn is_none(&self) -> bool {
        return matches!(self, MouseInputMode::None);
    }
}
