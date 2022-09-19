use crate::graphics::Color;
use crate::interface::{ChangeEvent, Container, DimensionConstraint, ElementCell, MutableColorValue, SizeConstraint, StaticLabel};

pub trait PrototypeMutableElement {

    fn to_mutable_element(&self, display: String, change_event: Option<ChangeEvent>) -> ElementCell;
}

impl PrototypeMutableElement for Color {

    fn to_mutable_element(&self, display: String, change_event: Option<ChangeEvent>) -> ElementCell {

        let elements: Vec<ElementCell> = vec![
            cell!(StaticLabel::new(display.clone())),
            cell!(MutableColorValue::new(display, self as *const Color, change_event)),
        ];

        cell!(Container::new(elements, Container::DEFAULT_SIZE))
    }
}

impl PrototypeMutableElement for DimensionConstraint {

    fn to_mutable_element(&self, display: String, _change_event: Option<ChangeEvent>) -> ElementCell {

        let elements: Vec<ElementCell> = vec![cell!(StaticLabel::new(display))];

        cell!(Container::new(elements, Container::DEFAULT_SIZE))
    }
}

impl PrototypeMutableElement for SizeConstraint {

    fn to_mutable_element(&self, display: String, _change_event: Option<ChangeEvent>) -> ElementCell {

        let elements: Vec<ElementCell> = vec![cell!(StaticLabel::new(display))];

        cell!(Container::new(elements, Container::DEFAULT_SIZE))
    }
}
