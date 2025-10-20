use crate::decibels::Decibels;

/// Settings for a static sound.
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct StaticSoundSettings {
    /// Sets if the sound should loop.
    pub(crate) loops: bool,
    /// The volume of the sound.
    pub(crate) volume: Decibels,
}

impl StaticSoundSettings {
    /// Creates a new [`StaticSoundSettings`] with the default settings.
    #[must_use]
    pub(crate) fn new() -> Self {
        Self {
            loops: false,
            volume: Decibels::IDENTITY,
        }
    }
}

impl Default for StaticSoundSettings {
    fn default() -> Self {
        Self::new()
    }
}
