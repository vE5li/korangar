use derive_new::new;
use serde::{ Serialize, Deserialize };

use interface::traits::{ PrototypeElement, PrototypeMutableElement, PrototypeMutableRangeElement };
use interface::ElementCell;

// TODO: rework when const generics are able to do this:
//pub struct MutableRange<T, const MIN: Vector2<T>, const MAX: Vector2<T>>(pub T);

#[derive(Serialize, Deserialize, new)]
pub struct MutableRange<T: Copy + PrototypeMutableRangeElement<T>> {
    pub inner: T,
    pub minimum: T, 
    pub maximum: T, 
}

impl<T: Copy + PrototypeMutableRangeElement<T>> PrototypeElement for MutableRange<T> {

    fn to_element(&self, display: String) -> ElementCell {
        self.inner.to_mutable_range_element(display, self.minimum, self.maximum)
    }
}

impl<T: Copy + PrototypeMutableRangeElement<T>> PrototypeMutableElement for MutableRange<T> {

    fn to_mutable_element(&self, display: String) -> ElementCell {
        self.inner.to_mutable_range_element(display, self.minimum, self.maximum)
    }
}

impl<T: Copy + PrototypeMutableRangeElement<T>> std::ops::Deref for MutableRange<T> {

    type Target = T;
   
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
