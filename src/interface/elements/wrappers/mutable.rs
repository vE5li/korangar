use std::marker::PhantomData;

use derive_new::new;
use serde::{Deserialize, Serialize};

use crate::interface::{ChangeEvent, ElementCell, IntoChangeEvent, PrototypeElement, PrototypeMutableElement};

#[derive(Serialize, Deserialize, new)]
pub struct Mutable<T: PrototypeMutableElement, E: IntoChangeEvent> {
    data: T,
    #[new(default)]
    _phantom_data: PhantomData<E>,
}

//impl<T: PrototypeMutableElement + Default> Default for Mutable<T> {
//
//    fn default() -> Self {
//        Self(T::default(), None)
//    }
//}

impl<T: PrototypeMutableElement, E: IntoChangeEvent> PrototypeElement for Mutable<T, E> {
    fn to_element(&self, display: String) -> ElementCell {
        self.data.to_mutable_element(display, E::into_change_event())
    }
}

impl<T: PrototypeMutableElement, E: IntoChangeEvent> PrototypeMutableElement for Mutable<T, E> {
    fn to_mutable_element(&self, display: String, _change_event: Option<ChangeEvent>) -> ElementCell {
        self.data.to_mutable_element(display, E::into_change_event())
    }
}

impl<T: PrototypeMutableElement, E: IntoChangeEvent> std::ops::Deref for Mutable<T, E> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}
