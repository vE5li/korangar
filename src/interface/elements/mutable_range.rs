use num::{ Zero, NumCast };
use num::traits::NumOps;
use cgmath::{ Vector2, Vector3, Vector4 };
use std::fmt::Display;

use crate::interface::*;
use crate::interface::{ ElementCell, ChangeEvent };

pub trait PrototypeMutableRangeElement<T> {

    fn to_mutable_range_element(&self, display: String, minimum: Self, maximum: Self, change_event: Option<ChangeEvent>) -> ElementCell;
}

impl<T> PrototypeMutableRangeElement<T> for f32 {

    fn to_mutable_range_element(&self, display: String, minimum: Self, maximum: Self, change_event: Option<ChangeEvent>) -> ElementCell {
        
        let elements: Vec<ElementCell> = vec![
            cell!(StaticLabel::new(display.clone())),
            cell!(MutableNumberValue::new(display, self as *const f32, minimum, maximum, change_event)),
        ];

        cell!(Container::new(elements, Container::DEFAULT_SIZE))
    }
}

impl<T: Zero + NumOps + NumCast + Copy + PartialOrd + Display + 'static> PrototypeMutableRangeElement<Vector2<T>> for Vector2<T> {

    fn to_mutable_range_element(&self, display: String, minimum: Self, maximum: Self, change_event: Option<ChangeEvent>) -> ElementCell {
        
        let elements: Vec<ElementCell> = vec![
            cell!(StaticLabel::new(display.clone())),
            cell!(MutableVector2Value::new(display, self as *const Self, minimum, maximum, change_event)),
        ];

        cell!(Container::new(elements, Container::DEFAULT_SIZE))
    }
}

impl<T: Zero + NumOps + NumCast + Copy + PartialOrd + Display + 'static> PrototypeMutableRangeElement<Vector3<T>> for Vector3<T> {

    fn to_mutable_range_element(&self, display: String, minimum: Self, maximum: Self, change_event: Option<ChangeEvent>) -> ElementCell {
        
        let elements: Vec<ElementCell> = vec![
            cell!(StaticLabel::new(display.clone())),
            cell!(MutableVector3Value::new(display, self as *const Self, minimum, maximum, change_event)),
        ];

        cell!(Container::new(elements, Container::DEFAULT_SIZE))
    }
}

impl<T: Zero + NumOps + NumCast + Copy + PartialOrd + Display + 'static> PrototypeMutableRangeElement<Vector4<T>> for Vector4<T> {

    fn to_mutable_range_element(&self, display: String, minimum: Self, maximum: Self, change_event: Option<ChangeEvent>) -> ElementCell {
        
        let elements: Vec<ElementCell> = vec![
            cell!(StaticLabel::new(display.clone())),
            cell!(MutableVector4Value::new(display, self as *const Self, minimum, maximum, change_event)),
        ];

        cell!(Container::new(elements, Container::DEFAULT_SIZE))
    }
}
