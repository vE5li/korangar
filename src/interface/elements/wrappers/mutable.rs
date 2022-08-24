use derive_new::new;
use serde::{Deserialize, Serialize};

use crate::interface::{ChangeEvent, ElementCell, PrototypeElement, PrototypeMutableElement};

#[derive(Serialize, Deserialize, new)]
pub struct Mutable<T: PrototypeMutableElement, const E: Option<ChangeEvent>>(pub T);

//impl<T: PrototypeMutableElement + Default> Default for Mutable<T> {
//
//    fn default() -> Self {
//        Self(T::default(), None)
//    }
//}

impl<T: PrototypeMutableElement, const E: Option<ChangeEvent>> PrototypeElement for Mutable<T, E> {

    fn to_element(&self, display: String) -> ElementCell {
        self.0.to_mutable_element(display, E)
    }
}

impl<T: PrototypeMutableElement, const E: Option<ChangeEvent>> PrototypeMutableElement for Mutable<T, E> {

    fn to_mutable_element(&self, display: String, _change_event: Option<ChangeEvent>) -> ElementCell {
        self.0.to_mutable_element(display, E)
    }
}

impl<T: PrototypeMutableElement, const E: Option<ChangeEvent>> std::ops::Deref for Mutable<T, E> {

    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
