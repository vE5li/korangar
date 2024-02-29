use crate::graphics::Color;
use crate::interface::*;

pub trait PrototypeMutableElement {
    fn to_mutable_element(&self, display: String, change_event: Option<ChangeEvent>) -> ElementCell;
}

impl PrototypeMutableElement for Color {
    fn to_mutable_element(&self, display: String, change_event: Option<ChangeEvent>) -> ElementCell {
        let static_self = unsafe { std::mem::transmute::<_, &'static Self>(self) };

        let elements = vec![
            StaticLabel::new(display.clone()).wrap(),
            MutableColorValue::new(display, static_self, change_event).wrap(),
        ];

        Container::new(elements).wrap()
    }
}

impl PrototypeMutableElement for DimensionConstraint {
    fn to_mutable_element(&self, display: String, _change_event: Option<ChangeEvent>) -> ElementCell {
        let elements = vec![StaticLabel::new(display).wrap()];

        Container::new(elements).wrap()
    }
}

impl PrototypeMutableElement for SizeConstraint {
    fn to_mutable_element(&self, display: String, _change_event: Option<ChangeEvent>) -> ElementCell {
        let elements = vec![StaticLabel::new(display).wrap()];

        Container::new(elements).wrap()
    }
}
