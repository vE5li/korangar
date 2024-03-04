use super::{StateButton, StateSelector};
use crate::interface::builder::{Set, Unset, With};
use crate::interface::*;

/// Type state [`StateButton`] builder. This builder utilizes the type system to
/// prevent calling the same method multiple times and calling
/// [`build`](Self::build) before the mandatory methods have been called.
#[must_use = "`build` needs to be called"]
pub struct StateButtonBuilder<TEXT, EVENT, SELECTOR, BACKGROUND, WIDTH> {
    text: TEXT,
    event: EVENT,
    selector: SELECTOR,
    transparent_background: bool,
    width_bound: Option<DimensionBound>,
    marker: PhantomData<(SELECTOR, BACKGROUND, WIDTH)>,
}

impl StateButtonBuilder<Unset, Unset, Unset, Unset, Unset> {
    pub fn new() -> Self {
        Self {
            text: Unset,
            event: Unset,
            selector: Unset,
            transparent_background: false,
            width_bound: None,
            marker: PhantomData,
        }
    }
}

impl<EVENT, SELECTOR, BACKGROUND, WIDTH> StateButtonBuilder<Unset, EVENT, SELECTOR, BACKGROUND, WIDTH> {
    pub fn with_text<TEXT: AsRef<str> + 'static>(self, text: TEXT) -> StateButtonBuilder<With<TEXT>, EVENT, SELECTOR, BACKGROUND, WIDTH> {
        StateButtonBuilder {
            text: With::new(text),
            ..self
        }
    }
}

impl<TEXT, SELECTOR, BACKGROUND, WIDTH> StateButtonBuilder<TEXT, Unset, SELECTOR, BACKGROUND, WIDTH> {
    pub fn with_event<EVENT: ElementEvent + 'static>(
        self,
        event: EVENT,
    ) -> StateButtonBuilder<TEXT, With<EVENT>, SELECTOR, BACKGROUND, WIDTH> {
        StateButtonBuilder {
            event: With::new(event),
            ..self
        }
    }
}

impl<TEXT, EVENT, BACKGROUND, WIDTH> StateButtonBuilder<TEXT, EVENT, Unset, BACKGROUND, WIDTH> {
    pub fn with_selector(
        self,
        selector: impl Fn(&StateProvider) -> bool + 'static,
    ) -> StateButtonBuilder<TEXT, EVENT, With<StateSelector>, BACKGROUND, WIDTH> {
        StateButtonBuilder {
            selector: With::new(Box::new(selector)),
            marker: PhantomData,
            ..self
        }
    }
}

impl<TEXT, EVENT, SELECTOR, WIDTH> StateButtonBuilder<TEXT, EVENT, SELECTOR, Unset, WIDTH> {
    pub fn with_transparent_background(self) -> StateButtonBuilder<TEXT, EVENT, SELECTOR, Set, WIDTH> {
        StateButtonBuilder {
            transparent_background: true,
            marker: PhantomData,
            ..self
        }
    }
}

impl<TEXT, EVENT, SELECTOR, BACKGROUND> StateButtonBuilder<TEXT, EVENT, SELECTOR, BACKGROUND, Unset> {
    pub fn with_width_bound(self, width_bound: DimensionBound) -> StateButtonBuilder<TEXT, EVENT, SELECTOR, BACKGROUND, Set> {
        StateButtonBuilder {
            width_bound: Some(width_bound),
            marker: PhantomData,
            ..self
        }
    }
}

impl<TEXT, EVENT, BACKGROUND, WIDTH> StateButtonBuilder<With<TEXT>, With<EVENT>, With<StateSelector>, BACKGROUND, WIDTH>
where
    TEXT: AsRef<str> + 'static,
    EVENT: ElementEvent + 'static,
{
    /// Take the builder and turn it into a [`StateButton`].
    ///
    /// NOTE: This method is only available if [`with_text`](Self::with_text),
    /// [`with_event`](Self::with_event), and
    /// [`with_selector`](Self::with_selector) have been called on
    /// the builder.
    pub fn build(self) -> StateButton<TEXT, EVENT> {
        let Self {
            text,
            event,
            selector,
            transparent_background,
            width_bound,
            ..
        } = self;

        let text = text.take();
        let event = event.take();
        let selector = selector.take();

        StateButton {
            text,
            event,
            selector,
            transparent_background,
            width_bound,
            state: Default::default(),
        }
    }
}
