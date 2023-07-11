use std::marker::PhantomData;

use derive_new::new;
use serde::{Deserialize, Serialize};

use crate::interface::{
    ChangeEvent, ElementCell, IntoChangeEvent, PrototypeElement, PrototypeMutableElement, PrototypeMutableRangeElement,
};

// TODO: rework when const generics are able to do this:
//pub struct MutableRange<T, const MIN: Vector2<T>, const MAX: Vector2<T>>(pub
// T);

#[derive(Serialize, Deserialize, new)]
pub struct MutableRange<T: Copy + PrototypeMutableRangeElement<T>, E: IntoChangeEvent> {
    inner: T,
    minimum: T,
    maximum: T,
    #[new(default)]
    _phantom_data: PhantomData<E>,
}

impl<T: Copy + PrototypeMutableRangeElement<T>, E: IntoChangeEvent> PrototypeElement for MutableRange<T, E> {
    fn to_element(&self, display: String) -> ElementCell {
        self.inner
            .to_mutable_range_element(display, self.minimum, self.maximum, E::into_change_event())
    }
}

impl<T: Copy + PrototypeMutableRangeElement<T>, E: IntoChangeEvent> PrototypeMutableElement for MutableRange<T, E> {
    fn to_mutable_element(&self, display: String, _change_event: Option<ChangeEvent>) -> ElementCell {
        self.inner
            .to_mutable_range_element(display, self.minimum, self.maximum, E::into_change_event())
    }
}

impl<T: Copy + PrototypeMutableRangeElement<T>, E: IntoChangeEvent> std::ops::Deref for MutableRange<T, E> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
