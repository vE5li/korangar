use interface::ElementCell;

pub enum HoverInformation {
    Element(ElementCell),
    Ignored,
    Hovered,
    Missed,
}
