use std::sync::Arc;

use super::TrackShared;
use crate::backend::resources::ResourceController;
use crate::error::PlaySoundError;
use crate::sound::{Sound, SoundData};

/// Controls a mixer track.
///
/// When a [`SpatialTrackHandle`] is dropped, the corresponding mixer
/// track will be removed.
pub(crate) struct SpatialTrackHandle {
    pub(crate) backend_sample_rate: u32,
    pub(crate) shared: Arc<TrackShared>,
    pub(crate) sound_controller: ResourceController<Box<dyn Sound>>,
}

impl SpatialTrackHandle {
    /// Plays a sound.
    pub(crate) fn play<D: SoundData>(&mut self, sound_data: D) -> Result<D::Handle, PlaySoundError<D::Error>> {
        let (sound, handle) = sound_data
            .into_sound(self.backend_sample_rate)
            .map_err(PlaySoundError::IntoSoundError)?;
        self.sound_controller.insert(sound).map_err(|_| PlaySoundError::SoundLimitReached)?;
        Ok(handle)
    }
}

impl Drop for SpatialTrackHandle {
    fn drop(&mut self) {
        self.shared.mark_for_removal();
    }
}
