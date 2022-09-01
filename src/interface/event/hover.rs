use crate::interface::ElementCell;

pub enum HoverInformation {
    Element(ElementCell),
    Hovered,
    Missed,
}
