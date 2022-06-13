use std::fmt::Display;

use types::Version;
use graphics::Color;
use types::maths::*;
use interface::elements::*;
use interface::types::{ ElementCell, SizeConstraint };

pub trait PrototypeElement {

    fn to_element(&self, display: String) -> ElementCell;
}

impl PrototypeElement for SizeConstraint {

    fn to_element(&self, display: String) -> ElementCell {

        let elements: Vec<ElementCell> = vec![
            cell!(StaticLabel::new(display.clone())),
        ];

        cell!(Container::new(elements, Container::DEFAULT_SIZE))
    }
}

impl<T: PrototypeElement + Copy + Display + 'static> PrototypeElement for Vector2<T> {

    fn to_element(&self, display: String) -> ElementCell {

        let elements: Vec<ElementCell> = vec![
            cell!(StaticLabel::new(display.clone())),
            cell!(Vector2Value::new(*self)),
        ];

        cell!(Container::new(elements, Container::DEFAULT_SIZE))
    }
}

impl<T: PrototypeElement + Copy + Display + 'static> PrototypeElement for Vector3<T> {

    fn to_element(&self, display: String) -> ElementCell {

        let elements: Vec<ElementCell> = vec![
            cell!(StaticLabel::new(display.clone())),
            cell!(Vector3Value::new(*self)),
        ];

        cell!(Container::new(elements, Container::DEFAULT_SIZE))
    }
}

impl<T: PrototypeElement> PrototypeElement for cgmath::Rad<T> {

    fn to_element(&self, display: String) -> ElementCell {
        self.0.to_element(display)
    }
}

impl<T: PrototypeElement> PrototypeElement for std::sync::Arc<T> {

    fn to_element(&self, display: String) -> ElementCell {
        self.as_ref().to_element(display)
    }
}

impl<T: PrototypeElement> PrototypeElement for Option<T> {

    fn to_element(&self, display: String) -> ElementCell {

        if let Some(value) = self {
            return value.to_element(display);
        }

        let elements: Vec<ElementCell> = vec![
            cell!(StaticLabel::new(display.clone())),
            cell!(StringValue::new("none".to_string())),
        ];

        cell!(Container::new(elements, Container::DEFAULT_SIZE))
    }
}

impl<T: PrototypeElement> PrototypeElement for Vec<T> {

    fn to_element(&self, display: String) -> ElementCell {

        let elements = self
            .iter()
            .enumerate()
            .map(|(index, item)| item.to_element(index.to_string()))
            .collect();

        cell!(Expandable::new(display, elements, false))
    }
}

impl PrototypeElement for Color {

    fn to_element(&self, display: String) -> ElementCell {

        let elements: Vec<ElementCell> = vec![
            cell!(StaticLabel::new(display)),
            cell!(ColorValue::new(*self)),
        ];

        cell!(Container::new(elements, Container::DEFAULT_SIZE))
    }
}

macro_rules! implement_prototype_element {
    ($ty:ident) => {
        impl PrototypeElement for $ty {

            fn to_element(&self, display: String) -> ElementCell {

                let elements: Vec<ElementCell> = vec![
                    cell!(StaticLabel::new(display.clone())),
                    cell!(StringValue::new(self.to_string())),
                ];

                cell!(Container::new(elements, Container::DEFAULT_SIZE))
            }
        }
    };
}

implement_prototype_element!(u8);
implement_prototype_element!(u16);
implement_prototype_element!(u32);
implement_prototype_element!(u64);
implement_prototype_element!(u128);

implement_prototype_element!(i8);
implement_prototype_element!(i16);
implement_prototype_element!(i32);
implement_prototype_element!(i64);
implement_prototype_element!(i128);

implement_prototype_element!(f32);
implement_prototype_element!(f64);

implement_prototype_element!(usize);
implement_prototype_element!(isize);

implement_prototype_element!(bool);

implement_prototype_element!(String);
implement_prototype_element!(Version);
