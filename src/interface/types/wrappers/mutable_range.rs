use derive_new::new;
use serde::{ Serialize, Deserialize };

use crate::interface::traits::{ PrototypeElement, PrototypeMutableElement, PrototypeMutableRangeElement };
use crate::interface::{ ElementCell, ChangeEvent };

// TODO: rework when const generics are able to do this:
//pub struct MutableRange<T, const MIN: Vector2<T>, const MAX: Vector2<T>>(pub T);

#[derive(Serialize, Deserialize, new)]
pub struct MutableRange<T: Copy + PrototypeMutableRangeElement<T>, const E: Option<ChangeEvent>> {
    inner: T,
    minimum: T, 
    maximum: T, 
}

impl<T: Copy + PrototypeMutableRangeElement<T>, const E: Option<ChangeEvent>> PrototypeElement for MutableRange<T, E> {

    fn to_element(&self, display: String) -> ElementCell {
        self.inner.to_mutable_range_element(display, self.minimum, self.maximum, E)
    }
}

impl<T: Copy + PrototypeMutableRangeElement<T>, const E: Option<ChangeEvent>> PrototypeMutableElement for MutableRange<T, E> {

    fn to_mutable_element(&self, display: String, _change_event: Option<ChangeEvent>) -> ElementCell {
        self.inner.to_mutable_range_element(display, self.minimum, self.maximum, E)
    }
}

impl<T: Copy + PrototypeMutableRangeElement<T>, const E: Option<ChangeEvent>> std::ops::Deref for MutableRange<T, E> {

    type Target = T;
   
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
