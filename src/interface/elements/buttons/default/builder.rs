use super::Button;
use crate::interface::builder::{Set, Unset};
use crate::interface::*;

/// Type state [`Button`] builder. This builder utilizes the type system to
/// prevent calling the same method multiple times and calling
/// [`build`](Self::build) before the mandatory methods have been called.
#[must_use = "`build` needs to be called"]
pub struct ButtonBuilder<TEXT, EVENT, DISABLED, FOREGROUND, BACKGROUND, WIDTH> {
    text: TEXT,
    event: EVENT,
    disabled_selector: Option<Selector>,
    foreground_color: Option<ColorSelector>,
    background_color: Option<ColorSelector>,
    width_bound: Option<DimensionBound>,
    marker: PhantomData<(DISABLED, FOREGROUND, BACKGROUND, WIDTH)>,
}

impl ButtonBuilder<Unset, Unset, Unset, Unset, Unset, Unset> {
    pub fn new() -> Self {
        Self {
            text: Unset,
            event: Unset,
            disabled_selector: None,
            foreground_color: None,
            background_color: None,
            width_bound: None,
            marker: PhantomData,
        }
    }
}

impl<EVENT, DISABLED, FOREGROUND, BACKGROUND, WIDTH> ButtonBuilder<Unset, EVENT, DISABLED, FOREGROUND, BACKGROUND, WIDTH> {
    pub fn with_text<TEXT: AsRef<str> + 'static>(self, text: TEXT) -> ButtonBuilder<TEXT, EVENT, DISABLED, FOREGROUND, BACKGROUND, WIDTH> {
        ButtonBuilder { text, ..self }
    }
}

impl<TEXT, DISABLED, FOREGROUND, BACKGROUND, WIDTH> ButtonBuilder<TEXT, Unset, DISABLED, FOREGROUND, BACKGROUND, WIDTH> {
    pub fn with_event<EVENT: ElementEvent + 'static>(
        self,
        event: EVENT,
    ) -> ButtonBuilder<TEXT, EVENT, DISABLED, FOREGROUND, BACKGROUND, WIDTH> {
        ButtonBuilder { event, ..self }
    }
}

impl<TEXT, EVENT, FOREGROUND, BACKGROUND, WIDTH> ButtonBuilder<TEXT, EVENT, Unset, FOREGROUND, BACKGROUND, WIDTH> {
    pub fn with_disabled_selector(
        self,
        selector: impl Fn() -> bool + 'static,
    ) -> ButtonBuilder<TEXT, EVENT, Set, FOREGROUND, BACKGROUND, WIDTH> {
        ButtonBuilder {
            disabled_selector: Some(Box::new(selector)),
            marker: PhantomData,
            ..self
        }
    }
}

impl<TEXT, EVENT, DISABLED, BACKGROUND, WIDTH> ButtonBuilder<TEXT, EVENT, DISABLED, Unset, BACKGROUND, WIDTH> {
    pub fn with_foreground_color(
        self,
        color_selector: impl Fn(&InterfaceTheme) -> Color + 'static,
    ) -> ButtonBuilder<TEXT, EVENT, DISABLED, Set, BACKGROUND, WIDTH> {
        ButtonBuilder {
            foreground_color: Some(Box::new(color_selector)),
            marker: PhantomData,
            ..self
        }
    }
}

impl<TEXT, EVENT, DISABLED, FOREGROUND, WIDTH> ButtonBuilder<TEXT, EVENT, DISABLED, FOREGROUND, Unset, WIDTH> {
    pub fn with_background_color(
        self,
        color_selector: impl Fn(&InterfaceTheme) -> Color + 'static,
    ) -> ButtonBuilder<TEXT, EVENT, DISABLED, FOREGROUND, Set, WIDTH> {
        ButtonBuilder {
            background_color: Some(Box::new(color_selector)),
            marker: PhantomData,
            ..self
        }
    }
}

impl<TEXT, EVENT, DISABLED, FOREGROUND, BACKGROUND> ButtonBuilder<TEXT, EVENT, DISABLED, FOREGROUND, BACKGROUND, Unset> {
    pub fn with_width_bound(self, width_bound: DimensionBound) -> ButtonBuilder<TEXT, EVENT, DISABLED, FOREGROUND, BACKGROUND, Set> {
        ButtonBuilder {
            width_bound: Some(width_bound),
            marker: PhantomData,
            ..self
        }
    }
}

impl<TEXT, EVENT, DISABLED, FOREGROUND, BACKGROUND, WIDTH> ButtonBuilder<TEXT, EVENT, DISABLED, FOREGROUND, BACKGROUND, WIDTH>
where
    TEXT: AsRef<str> + 'static,
    EVENT: ElementEvent + 'static,
{
    /// Take the builder and turn it into a [`Button`].
    ///
    /// NOTE: This method is only available if [`with_text`](Self::with_text)
    /// and [`with_event`](Self::with_event) have been called on
    /// the builder.
    pub fn build(self) -> Button<TEXT, EVENT> {
        let Self {
            text,
            event,
            disabled_selector,
            foreground_color,
            background_color,
            width_bound,
            ..
        } = self;

        Button {
            text,
            event,
            disabled_selector,
            foreground_color,
            background_color,
            width_bound,
            state: Default::default(),
        }
    }
}
