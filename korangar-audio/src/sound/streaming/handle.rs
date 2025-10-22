use std::sync::Arc;
use std::time::Duration;

use super::CommandWriters;
use super::sound::Shared;
use crate::sound::PlaybackState;

/// Controls a streaming sound.
pub(crate) struct StreamingSoundHandle {
    pub(super) shared: Arc<Shared>,
    pub(super) command_writers: CommandWriters,
}

impl StreamingSoundHandle {
    /// Returns the current playback state of the sound.
    #[must_use]
    pub(crate) fn state(&self) -> PlaybackState {
        self.shared.state()
    }

    /// Fades out the sound to silence with the given duration and then
    /// stops playback.
    ///
    /// Once the sound is stopped, it cannot be restarted.
    pub(crate) fn stop(&mut self, tween_duration: Duration) {
        self.command_writers.stop.write(tween_duration)
    }
}
