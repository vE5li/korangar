use std::marker::PhantomData;

use derive_new::new;
use serde::{Deserialize, Serialize};

use crate::interface::{ChangeEvent, ElementCell, IntoChangeEvent, PrototypeElement, PrototypeMutableElement};

#[derive(Serialize, Deserialize, new)]
pub struct Mutable<T, E>
where
    T: Copy + PrototypeMutableElement,
    E: IntoChangeEvent,
{
    data: T,
    #[new(default)]
    _phantom_data: PhantomData<E>,
}

impl<T, E> Mutable<T, E>
where
    T: Copy + PrototypeMutableElement,
    E: IntoChangeEvent,
{
    pub fn get(&self) -> T {
        self.data
    }
}

impl<T, E> PrototypeElement for Mutable<T, E>
where
    T: Copy + PrototypeMutableElement,
    E: IntoChangeEvent,
{
    fn to_element(&self, display: String) -> ElementCell {
        self.data.to_mutable_element(display, E::into_change_event())
    }
}

impl<T, E> PrototypeMutableElement for Mutable<T, E>
where
    T: Copy + PrototypeMutableElement,
    E: IntoChangeEvent,
{
    fn to_mutable_element(&self, display: String, _change_event: Option<ChangeEvent>) -> ElementCell {
        self.data.to_mutable_element(display, E::into_change_event())
    }
}
