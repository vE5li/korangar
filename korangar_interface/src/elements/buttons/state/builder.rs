use std::marker::PhantomData;

use super::StateButton;
use crate::ElementEvent;
use crate::application::Application;
use crate::builder::{Set, Unset};
use crate::layout::DimensionBound;
use crate::state::Remote;

/// Type state [`StateButton`] builder. This builder utilizes the type system to
/// prevent calling the same method multiple times and calling
/// [`build`](Self::build) before the mandatory methods have been called.
#[must_use = "`build` needs to be called"]
pub struct StateButtonBuilder<App, Text, Event, State, Background, Width>
where
    App: Application,
{
    text: Text,
    event: Event,
    remote: State,
    transparent_background: bool,
    width_bound: DimensionBound,
    marker: PhantomData<(App, State, Background, Width)>,
}

impl<App> StateButtonBuilder<App, Unset, Unset, Unset, Unset, Unset>
where
    App: Application,
{
    pub fn new() -> Self {
        Self {
            text: Unset,
            event: Unset,
            remote: Unset,
            transparent_background: false,
            width_bound: DimensionBound::RELATIVE_ONE_HUNDRED,
            marker: PhantomData,
        }
    }
}

impl<App, Event, State, Background, Width> StateButtonBuilder<App, Unset, Event, State, Background, Width>
where
    App: Application,
{
    pub fn with_text<Text: AsRef<str> + 'static>(self, text: Text) -> StateButtonBuilder<App, Text, Event, State, Background, Width> {
        StateButtonBuilder { text, ..self }
    }
}

impl<App, Text, State, Background, Width> StateButtonBuilder<App, Text, Unset, State, Background, Width>
where
    App: Application,
{
    pub fn with_event<Event: ElementEvent<App> + 'static>(
        self,
        event: Event,
    ) -> StateButtonBuilder<App, Text, Event, State, Background, Width> {
        StateButtonBuilder { event, ..self }
    }
}

impl<App, Text, Event, Background, Width> StateButtonBuilder<App, Text, Event, Unset, Background, Width>
where
    App: Application,
{
    pub fn with_remote<State>(self, remote: State) -> StateButtonBuilder<App, Text, Event, State, Background, Width>
    where
        State: Remote<bool> + 'static,
    {
        StateButtonBuilder {
            remote,
            marker: PhantomData,
            ..self
        }
    }
}

impl<App, Text, Event, State, Width> StateButtonBuilder<App, Text, Event, State, Unset, Width>
where
    App: Application,
{
    pub fn with_transparent_background(self) -> StateButtonBuilder<App, Text, Event, State, Set, Width> {
        StateButtonBuilder {
            transparent_background: true,
            marker: PhantomData,
            ..self
        }
    }
}

impl<App, Text, Event, State, Background> StateButtonBuilder<App, Text, Event, State, Background, Unset>
where
    App: Application,
{
    pub fn with_width_bound(self, width_bound: DimensionBound) -> StateButtonBuilder<App, Text, Event, State, Background, Set> {
        StateButtonBuilder {
            width_bound,
            marker: PhantomData,
            ..self
        }
    }
}

impl<App, Text, Event, State, Background, Width> StateButtonBuilder<App, Text, Event, State, Background, Width>
where
    App: Application,
    Text: AsRef<str> + 'static,
    Event: ElementEvent<App> + 'static,
    State: Remote<bool> + 'static,
{
    /// Take the builder and turn it into a [`StateButton`].
    ///
    /// NOTE: This method is only available if [`with_text`](Self::with_text),
    /// [`with_event`](Self::with_event), and
    /// [`with_selector`](Self::with_selector) have been called on
    /// the builder.
    pub fn build(self) -> StateButton<App, Text, Event, State> {
        let Self {
            text,
            event,
            remote,
            transparent_background,
            width_bound,
            ..
        } = self;

        StateButton {
            text,
            event,
            remote,
            transparent_background,
            width_bound,
            state: Default::default(),
        }
    }
}
