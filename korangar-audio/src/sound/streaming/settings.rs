use crate::Decibels;

/// Settings for a streaming sound.
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct StreamingSoundSettings {
    /// Sets if the sound should loop.
    pub(crate) loops: bool,
    /// The volume of the sound.
    pub(crate) volume: Decibels,
}

impl StreamingSoundSettings {
    /// Creates a new [`StreamingSoundSettings`] with the default settings.
    #[must_use]
    pub(crate) fn new() -> Self {
        Self {
            loops: false,
            volume: Decibels::IDENTITY,
        }
    }
}

impl Default for StreamingSoundSettings {
    fn default() -> Self {
        Self::new()
    }
}
