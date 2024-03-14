use korangar_interface::elements::{Container, ElementCell, ElementWrap, StaticLabel};
use korangar_interface::event::ChangeEvent;
use korangar_interface::layout::{DimensionBound, SizeBound};

use super::MutableColorValue;
use crate::graphics::Color;
use crate::interface::application::InterfaceSettings;

pub trait PrototypeMutableElement {
    fn to_mutable_element(&self, display: String, change_event: Option<ChangeEvent>) -> ElementCell<InterfaceSettings>;
}

impl PrototypeMutableElement for Color {
    fn to_mutable_element(&self, display: String, change_event: Option<ChangeEvent>) -> ElementCell<InterfaceSettings> {
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
    fn to_mutable_element(&self, display: String, _change_event: Option<ChangeEvent>) -> ElementCell<InterfaceSettings> {
        let elements = vec![StaticLabel::new(display).wrap()];

        Container::new(elements).wrap()
    }
}

impl PrototypeMutableElement for SizeBound {
    fn to_mutable_element(&self, display: String, _change_event: Option<ChangeEvent>) -> ElementCell<InterfaceSettings> {
        let elements = vec![StaticLabel::new(display).wrap()];

        Container::new(elements).wrap()
    }
}
