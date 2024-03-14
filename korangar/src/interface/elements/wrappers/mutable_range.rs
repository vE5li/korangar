use std::marker::PhantomData;

use derive_new::new;
use korangar_interface::elements::{ElementCell, PrototypeElement};
use korangar_interface::event::{ChangeEvent, IntoChangeEvent};
use serde::{Deserialize, Serialize};

use crate::interface::application::InterfaceSettings;
use crate::interface::elements::{PrototypeMutableElement, PrototypeMutableRangeElement};

// TODO: rework when const generics are able to do this:
//pub struct MutableRange<T, const MIN: Vector2<T>, const MAX: Vector2<T>>(pub
// T);

#[derive(Serialize, Deserialize, new)]
pub struct MutableRange<T, E>
where
    T: Copy + PrototypeMutableRangeElement<T>,
    E: IntoChangeEvent,
{
    inner: T,
    minimum: T,
    maximum: T,
    #[new(default)]
    #[serde(skip)]
    _phantom_data: PhantomData<E>,
}

impl<T, E> MutableRange<T, E>
where
    T: Copy + PrototypeMutableRangeElement<T>,
    E: IntoChangeEvent,
{
    pub fn get(&self) -> T {
        self.inner
    }
}

impl<T, E> PrototypeElement<InterfaceSettings> for MutableRange<T, E>
where
    T: Copy + PrototypeMutableRangeElement<T>,
    E: IntoChangeEvent,
{
    fn to_element(&self, display: String) -> ElementCell<InterfaceSettings> {
        self.inner
            .to_mutable_range_element(display, self.minimum, self.maximum, E::into_change_event())
    }
}

impl<T, E> PrototypeMutableElement for MutableRange<T, E>
where
    T: Copy + PrototypeMutableRangeElement<T>,
    E: IntoChangeEvent,
{
    fn to_mutable_element(&self, display: String, _change_event: Option<ChangeEvent>) -> ElementCell<InterfaceSettings> {
        self.inner
            .to_mutable_range_element(display, self.minimum, self.maximum, E::into_change_event())
    }
}
