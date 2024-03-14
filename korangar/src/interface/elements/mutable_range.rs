use std::fmt::Display;

use korangar_interface::elements::{Container, ElementCell, ElementDisplay, ElementWrap, StaticLabel};
use korangar_interface::event::ChangeEvent;
use num::traits::NumOps;
use num::{NumCast, Zero};

use super::{MutableArrayValue, MutableNumberValue};
use crate::interface::application::InterfaceSettings;
use crate::interface::layout::{ArrayType, CornerRadius, ScreenPosition, ScreenSize};
use crate::loaders::{FontSize, Scaling};

pub trait PrototypeMutableRangeElement<T> {
    fn to_mutable_range_element(
        &self,
        display: String,
        minimum: Self,
        maximum: Self,
        change_event: Option<ChangeEvent>,
    ) -> ElementCell<InterfaceSettings>;
}

// workaround for not having negative trait bounds or better specialization
pub trait IsVector {}

impl !IsVector for f32 {}
impl IsVector for ScreenPosition {}
impl IsVector for ScreenSize {}
impl IsVector for CornerRadius {}
impl IsVector for FontSize {}
impl IsVector for Scaling {}

impl PrototypeMutableRangeElement<f32> for f32 {
    fn to_mutable_range_element(
        &self,
        display: String,
        minimum: Self,
        maximum: Self,
        change_event: Option<ChangeEvent>,
    ) -> ElementCell<InterfaceSettings> {
        // SAFETY: This is obviously unsafe, so one needs to make sure that the element
        // implementing `PrototypeMutableRangeElement` will be valid and pinned while
        // this element exists. Additionally, it should only be used in a debug
        // context.
        let static_self = unsafe { std::mem::transmute::<_, &'static Self>(self) };

        let elements = vec![
            StaticLabel::new(display.clone()).wrap(),
            MutableNumberValue::new(display, static_self, minimum, maximum, change_event).wrap(),
        ];

        Container::new(elements).wrap()
    }
}

impl<T> PrototypeMutableRangeElement<T> for T
where
    T: ArrayType + ElementDisplay + IsVector + Copy + PartialEq + 'static, // TODO: !f32 or something
    T::Element: Zero + NumOps + NumCast + Copy + PartialOrd + Display + 'static,
    [(); T::ELEMENT_COUNT]:,
{
    fn to_mutable_range_element(
        &self,
        display: String,
        minimum: Self,
        maximum: Self,
        change_event: Option<ChangeEvent>,
    ) -> ElementCell<InterfaceSettings> {
        // SAFETY: This is obviously unsafe, so one needs to make sure that the element
        // implementing `PrototypeMutableRangeElement` will be valid and pinned while
        // this element exists. Additionally, it should only be used in a debug
        // context.
        let static_self = unsafe { std::mem::transmute::<_, &'static Self>(self) };

        let elements = vec![
            StaticLabel::new(display.clone()).wrap(),
            MutableArrayValue::new(display, static_self, minimum, maximum, change_event).wrap(),
        ];

        Container::new(elements).wrap()
    }
}
