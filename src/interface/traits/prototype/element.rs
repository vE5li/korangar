use std::rc::Rc;
use std::fmt::Display;

use crate::types::Version;
use crate::graphics::{ Color, ModelVertexBuffer, Texture };
use crate::types::maths::*;
use crate::interface::elements::*;
use crate::interface::types::{ ElementCell, SizeConstraint };

pub trait PrototypeElement {

    fn to_element(&self, display: String) -> ElementCell;
}

impl PrototypeElement for SizeConstraint {

    fn to_element(&self, display: String) -> ElementCell {

        let elements: Vec<ElementCell> = vec![
            cell!(StaticLabel::new(display)),
        ];

        cell!(Container::new(elements, Container::DEFAULT_SIZE))
    }
}

impl<T: PrototypeElement + Copy + Display + 'static> PrototypeElement for Vector2<T> {

    fn to_element(&self, display: String) -> ElementCell {

        let elements: Vec<ElementCell> = vec![
            cell!(StaticLabel::new(display)),
            cell!(Vector2Value::new(*self)),
        ];

        cell!(Container::new(elements, Container::DEFAULT_SIZE))
    }
}

impl<T: PrototypeElement + Copy + Display + 'static> PrototypeElement for Vector3<T> {

    fn to_element(&self, display: String) -> ElementCell {

        let elements: Vec<ElementCell> = vec![
            cell!(StaticLabel::new(display)),
            cell!(Vector3Value::new(*self)),
        ];

        cell!(Container::new(elements, Container::DEFAULT_SIZE))
    }
}

impl<T: PrototypeElement + Copy + Display + 'static> PrototypeElement for Quaternion<T> {

    fn to_element(&self, display: String) -> ElementCell {

        let elements: Vec<ElementCell> = vec![
            cell!(StaticLabel::new(display)),
            cell!(QuaternionValue::new(*self)),
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
            cell!(StaticLabel::new(display)),
            cell!(StringValue::new("none".to_string())),
        ];

        cell!(Container::new(elements, Container::DEFAULT_SIZE))
    }
}

impl<T: PrototypeElement, const SIZE: usize> PrototypeElement for [T; SIZE] {

    fn to_element(&self, display: String) -> ElementCell {

        let elements = self
            .iter()
            .enumerate()
            .map(|(index, item)| item.to_element(index.to_string()))
            .collect();

        cell!(Expandable::new(display, elements, false))
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

impl<T: PrototypeElement> PrototypeElement for Rc<T> {

    fn to_element(&self, display: String) -> ElementCell {
        (**self).to_element(display)
    }
}

impl PrototypeElement for ModelVertexBuffer {

    fn to_element(&self, display: String) -> ElementCell {
        use vulkano::buffer::BufferAccess;

        let identifier = self.inner().buffer.key();
        let size = self.inner().buffer.size();

        let elements: Vec<ElementCell> = vec![
            cell!(StaticLabel::new(display)),
            cell!(StringValue::new(format!("{} ({})", identifier, size))),
        ];

        cell!(Container::new(elements, Container::DEFAULT_SIZE))
    }
}

impl PrototypeElement for Texture {

    fn to_element(&self, display: String) -> ElementCell {
        use vulkano::VulkanObject;
        use vulkano::Handle;

        let identifier = self.internal_object().as_raw();

        let elements: Vec<ElementCell> = vec![
            cell!(StaticLabel::new(display)),
            cell!(StringValue::new(format!("0x{:x}", identifier))),
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
