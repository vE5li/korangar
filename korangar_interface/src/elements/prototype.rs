use std::fmt::Display;
use std::rc::Rc;

pub use interface_procedural::PrototypeElement;

use super::{Container, ElementCell, ElementWrap, Expandable, StaticLabel, StringValue};
use crate::application::Application;
use crate::layout::{DimensionBound, SizeBound};

pub trait PrototypeElement<App>
where
    App: Application,
{
    fn to_element(&self, display: String) -> ElementCell<App>;
}

pub trait ElementDisplay {
    fn display(&self) -> String;
}

// workaround for not having negative trait bounds or better specialization
auto trait NoDisplay {}
impl !NoDisplay for f32 {}
#[cfg(feature = "cgmath")]
impl<T> !NoDisplay for cgmath::Point3<T> {}
#[cfg(feature = "cgmath")]
impl<T> !NoDisplay for cgmath::Vector2<T> {}
#[cfg(feature = "cgmath")]
impl<T> !NoDisplay for cgmath::Vector3<T> {}
#[cfg(feature = "cgmath")]
impl<T> !NoDisplay for cgmath::Vector4<T> {}
#[cfg(feature = "cgmath")]
impl<T> !NoDisplay for cgmath::Quaternion<T> {}
#[cfg(feature = "cgmath")]
impl<T> !NoDisplay for cgmath::Rad<T> {}

impl<T> ElementDisplay for T
where
    T: Display + NoDisplay,
{
    fn display(&self) -> String {
        self.to_string()
    }
}

impl ElementDisplay for f32 {
    fn display(&self) -> String {
        format!("{self:.1}")
    }
}

#[cfg(feature = "cgmath")]
impl<T: ElementDisplay> ElementDisplay for cgmath::Point3<T> {
    fn display(&self) -> String {
        format!("{}, {}, {}", self.x.display(), self.y.display(), self.z.display())
    }
}

#[cfg(feature = "cgmath")]
impl<T: ElementDisplay> ElementDisplay for cgmath::Vector2<T> {
    fn display(&self) -> String {
        format!("{}, {}", self.x.display(), self.y.display())
    }
}

#[cfg(feature = "cgmath")]
impl<T: ElementDisplay> ElementDisplay for cgmath::Vector3<T> {
    fn display(&self) -> String {
        format!("{}, {}, {}", self.x.display(), self.y.display(), self.z.display())
    }
}

#[cfg(feature = "cgmath")]
impl<T: ElementDisplay> ElementDisplay for cgmath::Vector4<T> {
    fn display(&self) -> String {
        format!(
            "{}, {}, {}, {}",
            self.x.display(),
            self.y.display(),
            self.z.display(),
            self.w.display()
        )
    }
}

#[cfg(feature = "cgmath")]
impl<T: ElementDisplay> ElementDisplay for cgmath::Quaternion<T> {
    fn display(&self) -> String {
        format!(
            "{:.1}, {:.1}, {:.1} - {:.1}",
            self.v.x.display(),
            self.v.y.display(),
            self.v.z.display(),
            self.s.display()
        )
    }
}

#[cfg(feature = "cgmath")]
impl<T: ElementDisplay> ElementDisplay for cgmath::Rad<T> {
    fn display(&self) -> String {
        self.0.display()
    }
}

// workaround for not having negative trait bounds or better specialization
auto trait NoPrototype {}
impl<T> !NoPrototype for std::sync::Arc<T> {}
impl<T> !NoPrototype for Option<T> {}
impl<T, const N: usize> !NoPrototype for [T; N] {}
impl<T> !NoPrototype for &[T] {}
impl<T> !NoPrototype for Vec<T> {}
impl<T> !NoPrototype for Rc<T> {}

impl NoPrototype for &str {}
impl NoPrototype for String {}

impl<App, T> PrototypeElement<App> for T
where
    App: Application,
    T: ElementDisplay + NoPrototype,
{
    fn to_element(&self, display: String) -> ElementCell<App> {
        let elements = vec![StaticLabel::new(display).wrap(), StringValue::new(self.display()).wrap()];

        Container::new(elements).wrap()
    }
}

impl<App> PrototypeElement<App> for DimensionBound
where
    App: Application,
{
    fn to_element(&self, display: String) -> ElementCell<App> {
        let elements = vec![StaticLabel::new(display).wrap()];

        Container::new(elements).wrap()
    }
}

impl<App> PrototypeElement<App> for SizeBound
where
    App: Application,
{
    fn to_element(&self, display: String) -> ElementCell<App> {
        let elements = vec![StaticLabel::new(display).wrap()];

        Container::new(elements).wrap()
    }
}

impl<App, T> PrototypeElement<App> for std::sync::Arc<T>
where
    App: Application,
    T: PrototypeElement<App>,
{
    fn to_element(&self, display: String) -> ElementCell<App> {
        self.as_ref().to_element(display)
    }
}

impl<App, T> PrototypeElement<App> for Option<T>
where
    App: Application,
    T: PrototypeElement<App>,
{
    fn to_element(&self, display: String) -> ElementCell<App> {
        if let Some(value) = self {
            return value.to_element(display);
        }

        let elements = vec![StaticLabel::new(display).wrap(), StringValue::new("none".to_string()).wrap()];

        Container::new(elements).wrap()
    }
}

impl<App, T> PrototypeElement<App> for &[T]
where
    App: Application,
    T: PrototypeElement<App>,
{
    fn to_element(&self, display: String) -> ElementCell<App> {
        let elements = self
            .iter()
            .enumerate()
            .map(|(index, item)| item.to_element(index.to_string()))
            .collect();

        Expandable::new(display, elements, false).wrap()
    }
}

impl<App, T, const SIZE: usize> PrototypeElement<App> for [T; SIZE]
where
    App: Application,
    T: PrototypeElement<App>,
{
    fn to_element(&self, display: String) -> ElementCell<App> {
        let elements = self
            .iter()
            .enumerate()
            .map(|(index, item)| item.to_element(index.to_string()))
            .collect();

        Expandable::new(display, elements, false).wrap()
    }
}

impl<App, T> PrototypeElement<App> for Vec<T>
where
    App: Application,
    T: PrototypeElement<App>,
{
    fn to_element(&self, display: String) -> ElementCell<App> {
        let elements = self
            .iter()
            .enumerate()
            .map(|(index, item)| item.to_element(index.to_string()))
            .collect();

        Expandable::new(display, elements, false).wrap()
    }
}

impl<App, T> PrototypeElement<App> for Rc<T>
where
    App: Application,
    T: PrototypeElement<App>,
{
    fn to_element(&self, display: String) -> ElementCell<App> {
        (**self).to_element(display)
    }
}
