use std::any::{Any, TypeId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ElementId(usize);

#[derive(Clone)]
pub(crate) struct ElementIdGenerator {
    next_free_id: usize,
}

impl ElementIdGenerator {
    pub fn new() -> Self {
        Self { next_free_id: 0 }
    }

    pub fn generate(&mut self) -> ElementId {
        let id = ElementId(self.next_free_id);

        self.next_free_id += 1;

        id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FocusId(TypeId);

pub trait FocusIdExt {
    fn focus_id(&self) -> FocusId;
}

impl<T> FocusIdExt for T
where
    T: Any,
{
    fn focus_id(&self) -> FocusId {
        FocusId(self.type_id())
    }
}
