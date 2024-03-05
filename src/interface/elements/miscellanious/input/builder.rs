use std::fmt::Display;

use procedural::dimension_bound;

use super::{EnterAction, InputField};
use crate::interface::builder::{Set, Unset};
use crate::interface::*;

/// Type state [`InputField`] builder. This builder utilizes the type system to
/// prevent calling the same method multiple times and calling
/// [`build`](Self::build) before the mandatory methods have been called.
#[must_use = "`build` needs to be called"]
pub struct InputFieldBuilder<STATE, TEXT, ACTION, LENGTH, HIDDEN, WIDTH> {
    input_state: STATE,
    ghost_text: TEXT,
    enter_action: ACTION,
    length: usize,
    hidden: bool,
    width_bound: DimensionBound,
    marker: PhantomData<(LENGTH, HIDDEN, WIDTH)>,
}

impl InputFieldBuilder<Unset, Unset, Unset, Unset, Unset, Unset> {
    pub fn new() -> Self {
        Self {
            input_state: Unset,
            ghost_text: Unset,
            enter_action: Unset,
            length: 0,
            hidden: false,
            width_bound: dimension_bound!(100%),
            marker: PhantomData,
        }
    }
}

impl<TEXT, ACTION, LENGTH, HIDDEN, WIDTH> InputFieldBuilder<Unset, TEXT, ACTION, LENGTH, HIDDEN, WIDTH> {
    pub fn with_state(self, state: TrackedState<String>) -> InputFieldBuilder<TrackedState<String>, TEXT, ACTION, LENGTH, HIDDEN, WIDTH> {
        InputFieldBuilder {
            input_state: state,
            ..self
        }
    }
}

impl<STATE, ACTION, LENGTH, HIDDEN, WIDTH> InputFieldBuilder<STATE, Unset, ACTION, LENGTH, HIDDEN, WIDTH> {
    /// Set the text that will be displayed when the [`InputField`] is empty.
    pub fn with_ghost_text<TEXT: Display + 'static>(
        self,
        ghost_text: TEXT,
    ) -> InputFieldBuilder<STATE, TEXT, ACTION, LENGTH, HIDDEN, WIDTH> {
        InputFieldBuilder { ghost_text, ..self }
    }
}

impl<STATE, TEXT, LENGTH, HIDDEN, WIDTH> InputFieldBuilder<STATE, TEXT, Unset, LENGTH, HIDDEN, WIDTH> {
    /// Set an action that will be executed when the user presses the enter key.
    pub fn with_enter_action(
        self,
        enter_action: impl FnMut() -> Vec<ClickAction> + 'static,
    ) -> InputFieldBuilder<STATE, TEXT, EnterAction, LENGTH, HIDDEN, WIDTH> {
        InputFieldBuilder {
            enter_action: Box::new(enter_action),
            ..self
        }
    }
}

impl<STATE, TEXT, ACTION, HIDDEN, WIDTH> InputFieldBuilder<STATE, TEXT, ACTION, Unset, HIDDEN, WIDTH> {
    /// Set the maximum number of allowed characters.
    pub fn with_length(self, length: usize) -> InputFieldBuilder<STATE, TEXT, ACTION, Set, HIDDEN, WIDTH> {
        InputFieldBuilder {
            length,
            marker: PhantomData,
            ..self
        }
    }
}

impl<STATE, TEXT, ACTION, LENGTH, WIDTH> InputFieldBuilder<STATE, TEXT, ACTION, LENGTH, Unset, WIDTH> {
    /// Only show text as `*` characters. Useful for password fields.
    pub fn hidden(self) -> InputFieldBuilder<STATE, TEXT, ACTION, LENGTH, Set, WIDTH> {
        InputFieldBuilder {
            hidden: true,
            marker: PhantomData,
            ..self
        }
    }
}

impl<STATE, TEXT, ACTION, LENGTH, HIDDEN> InputFieldBuilder<STATE, TEXT, ACTION, LENGTH, HIDDEN, Unset> {
    pub fn with_width_bound(self, width_bound: DimensionBound) -> InputFieldBuilder<STATE, TEXT, ACTION, LENGTH, HIDDEN, Set> {
        InputFieldBuilder {
            width_bound,
            marker: PhantomData,
            ..self
        }
    }
}

impl<TEXT, HIDDEN, WIDTH> InputFieldBuilder<TrackedState<String>, TEXT, EnterAction, Set, HIDDEN, WIDTH>
where
    TEXT: Display + 'static,
{
    /// Take the builder and turn it into a [`InputField`].
    ///
    /// NOTE: This method is only available if [`with_state`](Self::with_state),
    /// [`with_ghost_text`](Self::with_ghost_text),
    /// [`with_enter_action`](Self::with_enter_action),
    /// and [`with_length`](Self::with_length) have been called on the builder.
    pub fn build(self) -> InputField<TEXT> {
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
