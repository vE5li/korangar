use std::sync::Arc;

use super::sound::Shared;
use crate::sound::PlaybackState;

/// Controls a static sound.
pub(crate) struct StaticSoundHandle {
    pub(super) shared: Arc<Shared>,
}

impl StaticSoundHandle {
    /// Returns the current playback state of the sound.
    #[must_use]
    pub(crate) fn state(&self) -> PlaybackState {
        self.shared.state()
    }
}
