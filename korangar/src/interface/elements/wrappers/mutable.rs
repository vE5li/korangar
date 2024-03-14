use std::marker::PhantomData;

use derive_new::new;
use korangar_interface::elements::{ElementCell, PrototypeElement};
use korangar_interface::event::{ChangeEvent, IntoChangeEvent};
use serde::{Deserialize, Serialize};

use crate::interface::application::InterfaceSettings;
use crate::interface::elements::PrototypeMutableElement;

#[derive(Serialize, Deserialize, new)]
pub struct Mutable<T, E>
where
    T: Copy + PrototypeMutableElement,
    E: IntoChangeEvent,
{
    data: T,
    #[new(default)]
    #[serde(skip)]
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

impl<T, E> PrototypeElement<InterfaceSettings> for Mutable<T, E>
where
    T: Copy + PrototypeMutableElement,
    E: IntoChangeEvent,
{
    fn to_element(&self, display: String) -> ElementCell<InterfaceSettings> {
        self.data.to_mutable_element(display, E::into_change_event())
    }
}

impl<T, E> PrototypeMutableElement for Mutable<T, E>
where
    T: Copy + PrototypeMutableElement,
    E: IntoChangeEvent,
{
    fn to_mutable_element(&self, display: String, _change_event: Option<ChangeEvent>) -> ElementCell<InterfaceSettings> {
        self.data.to_mutable_element(display, E::into_change_event())
    }
}
