use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Writes values that can be sent to a [`CommandReader`].
pub(crate) struct CommandWriter<T: Send + Copy>(Arc<Mutex<Option<T>>>);

impl<T: Send + Copy> CommandWriter<T> {
    /// Writes a new value, overwriting any previous values.
    pub(crate) fn write(&self, command: T) {
        self.0.lock().unwrap().replace(command);
    }
}

/// Reads values that were written to a [`CommandWriter`].
pub(crate) struct CommandReader<T: Send + Copy>(Arc<Mutex<Option<T>>>);

impl<T: Send + Copy> CommandReader<T> {
    /// Returns the latest value that was written to the [`CommandWriter`].
    #[must_use]
    pub(crate) fn read(&self) -> Option<T> {
        // We use try_lock() as to never block the audio thread.
        // This is fine, since we can read the command in the next frame.
        self.0.try_lock().ok()?.take()
    }
}

/// Creates a command writer/reader pair.
#[must_use]
pub(crate) fn command_writer_and_reader<T: Send + Copy>() -> (CommandWriter<T>, CommandReader<T>) {
    let state = Arc::new(Mutex::new(None));
    (CommandWriter(Arc::clone(&state)), CommandReader(state))
}

/// A command that holds a target value and a tween duration.
///
/// Setting a parameter to a value with a given duration is a common
/// pattern in the audio engine.
///
/// `CommandReader<ValueChangeCommand>`s can be passed to
/// [`Parameter`](crate::Parameter)s to quickly set the parameter to
/// a new value read from the [`CommandReader`].
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct ValueChangeCommand<T> {
    /// The new value to set something to.
    pub(crate) target: T,
    /// The duration to smoothly transition the value.
    pub(crate) tween_duration: Duration,
}
