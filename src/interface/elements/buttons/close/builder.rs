use super::CloseButton;

/// Type state [`CloseButton`] builder. This builder utilizes the type system to
/// prevent calling the same method multiple times and calling
/// [`build`](Self::build) before the mandatory methods have been called.
#[must_use = "`build` needs to be called"]
pub struct CloseButtonBuilder;

impl CloseButtonBuilder {
    pub fn new() -> Self {
        Self
    }

    /// Take the builder and turn it into a [`CloseButton`].
    pub fn build(self) -> CloseButton {
        CloseButton { state: Default::default() }
    }
}
