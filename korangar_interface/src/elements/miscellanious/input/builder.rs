use std::fmt::Display;
use std::marker::PhantomData;

use super::{EnterAction, InputField};
use crate::application::Application;
use crate::builder::{Set, Unset};
use crate::event::ClickAction;
use crate::layout::DimensionBound;
use crate::state::PlainTrackedState;

/// Type state [`InputField`] builder. This builder utilizes the type system to
/// prevent calling the same method multiple times and calling
/// [`build`](Self::build) before the mandatory methods have been called.
#[must_use = "`build` needs to be called"]
pub struct InputFieldBuilder<App, State, Text, Action, Length, Hidden, Width>
where
    App: Application,
{
    input_state: State,
    ghost_text: Text,
    enter_action: Action,
    length: usize,
    hidden: bool,
    width_bound: DimensionBound,
    marker: PhantomData<(App, Length, Hidden, Width)>,
}

impl<App> InputFieldBuilder<App, Unset, Unset, Unset, Unset, Unset, Unset>
where
    App: Application,
{
    pub fn new() -> Self {
        Self {
            input_state: Unset,
            ghost_text: Unset,
            enter_action: Unset,
            length: 0,
            hidden: false,
            width_bound: DimensionBound::RELATIVE_ONE_HUNDRED,
            marker: PhantomData,
        }
    }
}

impl<App, Text, Action, Length, Hidden, Width> InputFieldBuilder<App, Unset, Text, Action, Length, Hidden, Width>
where
    App: Application,
{
    pub fn with_state(
        self,
        state: PlainTrackedState<String>,
    ) -> InputFieldBuilder<App, PlainTrackedState<String>, Text, Action, Length, Hidden, Width> {
        InputFieldBuilder {
            input_state: state,
            ..self
        }
    }
}

impl<App, State, Action, Length, Hidden, Width> InputFieldBuilder<App, State, Unset, Action, Length, Hidden, Width>
where
    App: Application,
{
    /// Set the text that will be displayed when the [`InputField`] is empty.
    pub fn with_ghost_text<Text>(self, ghost_text: Text) -> InputFieldBuilder<App, State, Text, Action, Length, Hidden, Width>
    where
        Text: Display + 'static,
    {
        InputFieldBuilder { ghost_text, ..self }
    }
}

impl<App, State, Text, Length, Hidden, Width> InputFieldBuilder<App, State, Text, Unset, Length, Hidden, Width>
where
    App: Application,
{
    /// Set an action that will be executed when the user presses the enter key.
    pub fn with_enter_action(
        self,
        enter_action: impl FnMut() -> Vec<ClickAction<App>> + 'static,
    ) -> InputFieldBuilder<App, State, Text, EnterAction<App>, Length, Hidden, Width> {
        InputFieldBuilder {
            enter_action: Box::new(enter_action),
            ..self
        }
    }
}

impl<App, State, Text, Action, Hidden, Width> InputFieldBuilder<App, State, Text, Action, Unset, Hidden, Width>
where
    App: Application,
{
    /// Set the maximum number of allowed characters.
    pub fn with_length(self, length: usize) -> InputFieldBuilder<App, State, Text, Action, Set, Hidden, Width> {
        InputFieldBuilder {
            length,
            marker: PhantomData,
            ..self
        }
    }
}

impl<App, State, Text, Action, Length, Width> InputFieldBuilder<App, State, Text, Action, Length, Unset, Width>
where
    App: Application,
{
    /// Only show text as `*` characters. Useful for password fields.
    pub fn hidden(self) -> InputFieldBuilder<App, State, Text, Action, Length, Set, Width> {
        InputFieldBuilder {
            hidden: true,
            marker: PhantomData,
            ..self
        }
    }
}

impl<App, State, Text, Action, Length, Hidden> InputFieldBuilder<App, State, Text, Action, Length, Hidden, Unset>
where
    App: Application,
{
    pub fn with_width_bound(self, width_bound: DimensionBound) -> InputFieldBuilder<App, State, Text, Action, Length, Hidden, Set> {
        InputFieldBuilder {
            width_bound,
            marker: PhantomData,
            ..self
        }
    }
}

impl<App, Text, Hidden, Width> InputFieldBuilder<App, PlainTrackedState<String>, Text, EnterAction<App>, Set, Hidden, Width>
where
    App: Application,
    Text: Display + 'static,
{
    /// Take the builder and turn it into a [`InputField`].
    ///
    /// NOTE: This method is only available if [`with_state`](Self::with_state),
    /// [`with_ghost_text`](Self::with_ghost_text),
    /// [`with_enter_action`](Self::with_enter_action),
    /// and [`with_length`](Self::with_length) have been called on the builder.
    pub fn build(self) -> InputField<App, Text> {
        let Self {
            input_state,
            ghost_text,
            enter_action,
            length,
            hidden,
            width_bound,
            ..
        } = self;

        InputField {
            input_state,
            ghost_text,
            enter_action,
            length,
            hidden,
            width_bound,
            state: Default::default(),
        }
    }
}
