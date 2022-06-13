use graphics::Color;
use interface::elements::{ MutableColorValue, StaticLabel, Container };
use interface::types::{ ElementCell, SizeConstraint };

pub trait PrototypeMutableElement {

    fn to_mutable_element(&self, display: String) -> ElementCell;
}

impl PrototypeMutableElement for Color {

    fn to_mutable_element(&self, display: String) -> ElementCell {

        let elements: Vec<ElementCell> = vec![
            cell!(StaticLabel::new(display.clone())),
            cell!(MutableColorValue::new(display, self as *const Color)),
        ];

        cell!(Container::new(elements, Container::DEFAULT_SIZE))
    }
}

impl PrototypeMutableElement for SizeConstraint {

    fn to_mutable_element(&self, display: String) -> ElementCell {

        let elements: Vec<ElementCell> = vec![
            cell!(StaticLabel::new(display.clone())),
        ];

        cell!(Container::new(elements, Container::DEFAULT_SIZE))
    }
}
