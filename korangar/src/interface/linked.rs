use std::cell::UnsafeCell;

use korangar_interface::elements::ElementCell;

use crate::interface::application::InterfaceSettings;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LinkedElementInner {
    Set(usize),
    Hidden,
    Unset,
}

pub struct LinkedElement {
    inner: UnsafeCell<LinkedElementInner>,
}

impl LinkedElement {
    pub fn new() -> Self {
        Self {
            inner: UnsafeCell::new(LinkedElementInner::Unset),
        }
    }

    pub fn link(&self, element: &ElementCell<InterfaceSettings>) {
        let element_address = element.as_ptr() as *const () as usize;
        unsafe { *self.inner.get() = LinkedElementInner::Set(element_address) };
    }

    pub fn link_hidden(&self) {
        unsafe { *self.inner.get() = LinkedElementInner::Hidden };
    }

    pub fn is_linked(&self) -> bool {
        unsafe { *self.inner.get() != LinkedElementInner::Unset }
    }

    pub fn is_hidden(&self) -> bool {
        unsafe { *self.inner.get() == LinkedElementInner::Hidden }
    }

    pub fn is_linked_to(&self, element: &ElementCell<InterfaceSettings>) -> bool {
        unsafe { *self.inner.get() == LinkedElementInner::Set(element.as_ptr() as *const () as usize) }
    }
}
