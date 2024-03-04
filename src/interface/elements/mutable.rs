use crate::graphics::Color;
use crate::interface::*;

pub trait PrototypeMutableElement {
    fn to_mutable_element(&self, display: String, change_event: Option<ChangeEvent>) -> ElementCell;
}

impl PrototypeMutableElement for Color {
    fn to_mutable_element(&self, display: String, change_event: Option<ChangeEvent>) -> ElementCell {
        // SAFETY: This is obviously unsafe, so one needs to make sure that the element
        // implementing `PrototypeMutableElement` will be valid and pinned while this
        // element exists. Additionally, it should only be used in a debug
        // context.
        let static_self = unsafe { std::mem::transmute::<_, &'static Self>(self) };

        let elements = vec![
            StaticLabel::new(display.clone()).wrap(),
            MutableColorValue::new(display, static_self, change_event).wrap(),
        ];

        Container::new(elements).wrap()
    }
}

impl PrototypeMutableElement for DimensionBound {
    fn to_mutable_element(&self, display: String, _change_event: Option<ChangeEvent>) -> ElementCell {
        let elements = vec![StaticLabel::new(display).wrap()];

        Container::new(elements).wrap()
    }
}

impl PrototypeMutableElement for SizeBound {
    fn to_mutable_element(&self, display: String, _change_event: Option<ChangeEvent>) -> ElementCell {
        let elements = vec![StaticLabel::new(display).wrap()];

        Container::new(elements).wrap()
    }
}
