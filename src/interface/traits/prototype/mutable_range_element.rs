use num::{ Zero, NumCast };
use num::traits::NumOps;
use std::fmt::Display;

use types::maths::*;
use interface::elements::{ StaticLabel, Container };
use interface::elements::*;
use interface::types::ElementCell;

pub trait PrototypeMutableRangeElement<T> {

    fn to_mutable_range_element(&self, display: String, minimum: Self, maximum: Self) -> ElementCell;
}

impl<T> PrototypeMutableRangeElement<T> for f32 {

    fn to_mutable_range_element(&self, display: String, minimum: Self, maximum: Self) -> ElementCell {
        
        let elements: Vec<ElementCell> = vec![
            cell!(StaticLabel::new(display.clone())),
            cell!(MutableNumberValue::new(display, self as *const f32, minimum, maximum)),
        ];

        cell!(Container::new(elements, Container::DEFAULT_SIZE))
    }
}

impl<T: Zero + NumOps + NumCast + Copy + PartialOrd + Display + 'static> PrototypeMutableRangeElement<Vector2<T>> for Vector2<T> {

    fn to_mutable_range_element(&self, display: String, minimum: Self, maximum: Self) -> ElementCell {
        
        let elements: Vec<ElementCell> = vec![
            cell!(StaticLabel::new(display.clone())),
            cell!(MutableVector2Value::new(display, self as *const Self, minimum, maximum)),
        ];

        cell!(Container::new(elements, Container::DEFAULT_SIZE))
    }
}

impl<T: Zero + NumOps + NumCast + Copy + PartialOrd + Display + 'static> PrototypeMutableRangeElement<Vector4<T>> for Vector4<T> {

    fn to_mutable_range_element(&self, display: String, minimum: Self, maximum: Self) -> ElementCell {
        
        let elements: Vec<ElementCell> = vec![
            cell!(StaticLabel::new(display.clone())),
            cell!(MutableVector4Value::new(display, self as *const Self, minimum, maximum)),
        ];

        cell!(Container::new(elements, Container::DEFAULT_SIZE))
    }
}
