use std::fmt::Display;

use cgmath::{Array, Vector2, Vector3, Vector4};
use num::traits::NumOps;
use num::{NumCast, Zero};

use crate::interface::{ChangeEvent, ElementCell, *};

pub trait PrototypeMutableRangeElement<T> {

    fn to_mutable_range_element(&self, display: String, minimum: Self, maximum: Self, change_event: Option<ChangeEvent>) -> ElementCell;
}

// workaround for not having negative trait bounds or better specialization
pub trait IsVector {}

impl !IsVector for f32 {}
impl<T> IsVector for Vector2<T> {}
impl<T> IsVector for Vector3<T> {}
impl<T> IsVector for Vector4<T> {}

impl PrototypeMutableRangeElement<f32> for f32 {

    fn to_mutable_range_element(&self, display: String, minimum: Self, maximum: Self, change_event: Option<ChangeEvent>) -> ElementCell {

        let elements: Vec<ElementCell> = vec![
            cell!(StaticLabel::new(display.clone())),
            cell!(MutableNumberValue::new(
                display,
                self as *const f32,
                minimum,
                maximum,
                change_event
            )),
        ];

        cell!(Container::new(elements, Container::DEFAULT_SIZE))
    }
}

impl<T> PrototypeMutableRangeElement<T> for T
where
    T: Array + ElementDisplay + IsVector + Copy + PartialEq + 'static, // TODO: !f32 or something
    T::Element: Zero + NumOps + NumCast + Copy + PartialOrd + Display + 'static,
{

    fn to_mutable_range_element(&self, display: String, minimum: Self, maximum: Self, change_event: Option<ChangeEvent>) -> ElementCell {

        let elements: Vec<ElementCell> = vec![
            cell!(StaticLabel::new(display.clone())),
            cell!(MutableVectorValue::new(
                display,
                self as *const Self,
                minimum,
                maximum,
                change_event
            )),
        ];

        cell!(Container::new(elements, Container::DEFAULT_SIZE))
    }
}
