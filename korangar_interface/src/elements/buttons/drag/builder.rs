use super::DragButton;
use crate::application::Application;
use crate::builder::Unset;
use crate::layout::DimensionBound;

/// Type state [`DragButton`] builder. This builder utilizes the type system to
/// prevent calling the same method multiple times and calling
/// [`build`](Self::build) before the mandatory methods have been called.
#[must_use = "`build` needs to be called"]
pub struct DragButtonBuilder<Title, Width> {
    title: Title,
    width_bound: Width,
}

impl DragButtonBuilder<Unset, Unset> {
    pub fn new() -> Self {
        Self {
            title: Unset,
            width_bound: Unset,
        }
    }
}

impl<Width> DragButtonBuilder<Unset, Width> {
    pub fn with_title(self, title: impl Into<String>) -> DragButtonBuilder<String, Width> {
        DragButtonBuilder {
            title: title.into(),
            ..self
        }
    }
}

impl<Title> DragButtonBuilder<Title, Unset> {
    pub fn with_width_bound(self, width_bound: DimensionBound) -> DragButtonBuilder<Title, DimensionBound> {
        DragButtonBuilder { width_bound, ..self }
    }
}

impl DragButtonBuilder<String, DimensionBound> {
    /// Take the builder and turn it into a [`DragButton`].
    ///
    /// NOTE: This method is only available if [`with_title`](Self::with_title)
    /// and [`with_width_bound`](Self::with_width_bound) have been called on
    /// the builder.
    pub fn build<App>(self) -> DragButton<App>
    where
        App: Application,
    {
        let Self { title, width_bound } = self;

        DragButton {
            title,
            width_bound,
            state: Default::default(),
        }
    }
}
