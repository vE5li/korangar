use serde::{ Serialize, Deserialize };

use interface::traits::{ PrototypeElement, PrototypeMutableElement };
use interface::ElementCell;

#[derive(Serialize, Deserialize)]
pub struct Mutable<T: PrototypeMutableElement>(pub T);

impl<T: PrototypeMutableElement + Default> Default for Mutable<T> {

    fn default() -> Self {
        Self(T::default())
    } 
}

impl<T: PrototypeMutableElement> PrototypeElement for Mutable<T> {

    fn to_element(&self, display: String) -> ElementCell {
        self.0.to_mutable_element(display)
    }
}

impl<T: PrototypeMutableElement> PrototypeMutableElement for Mutable<T> {

    fn to_mutable_element(&self, display: String) -> ElementCell {
        self.0.to_mutable_element(display)
    }
}

impl<T: PrototypeMutableElement> std::ops::Deref for Mutable<T> {

    type Target = T;
   
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
