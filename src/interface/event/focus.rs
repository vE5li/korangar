use crate::interface::ElementCell;

pub enum FocusEvent {
    Focus(ElementCell),
    LeftClick(ElementCell),
}
