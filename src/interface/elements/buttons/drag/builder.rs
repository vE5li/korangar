use super::DragButton;
use crate::interface::builder::Unset;
use crate::interface::*;

/// Type state [`DragButton`] builder. This builder utilizes the type system to
/// prevent calling the same method multiple times and calling
/// [`build`](Self::build) before the mandatory methods have been called.
#[must_use = "`build` needs to be called"]
pub struct DragButtonBuilder<TITLE, WIDTH> {
    title: TITLE,
    width_bound: WIDTH,
}

impl DragButtonBuilder<Unset, Unset> {
    pub fn new() -> Self {
        Self {
            title: Unset,
            width_bound: Unset,
        }
    }
}

impl<WIDTH> DragButtonBuilder<Unset, WIDTH> {
    pub fn with_title(self, title: impl Into<String>) -> DragButtonBuilder<String, WIDTH> {
        DragButtonBuilder {
            title: title.into(),
            ..self
        }
    }
}

impl<TITLE> DragButtonBuilder<TITLE, Unset> {
    pub fn with_width_bound(self, width_bound: DimensionBound) -> DragButtonBuilder<TITLE, DimensionBound> {
        DragButtonBuilder { width_bound, ..self }
    }
}

impl DragButtonBuilder<String, DimensionBound> {
    /// Take the builder and turn it into a [`DragButton`].
    ///
    /// NOTE: This method is only available if [`with_title`](Self::with_title)
    /// and [`with_width_bound`](Self::with_width_bound) have been called on
    /// the builder.
    pub fn build(self) -> DragButton {
        let Self { title, width_bound } = self;

        DragButton {
            title,
            width_bound,
            state: Default::default(),
        }
    }
}
