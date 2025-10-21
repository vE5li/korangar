use std::sync::Arc;
use std::time::Duration;

use cgmath::Point3;

use super::{CommandWriters, SpatialTrackBuilder, SpatialTrackHandle, Track, TrackShared};
use crate::backend::RendererShared;
use crate::backend::resources::ResourceController;
use crate::command::ValueChangeCommand;
use crate::decibels::Decibels;
use crate::error::{PlaySoundError, ResourceLimitReached};
use crate::sound::{Sound, SoundData};

/// Controls a mixer track.
///
/// When a [`TrackHandle`] is dropped, the corresponding mixer track will be
/// removed.
pub(crate) struct TrackHandle {
    pub(crate) backend_sample_rate: u32,
    pub(crate) renderer_shared: Arc<RendererShared>,
    pub(crate) shared: Arc<TrackShared>,
    pub(crate) command_writers: CommandWriters,
    pub(crate) sound_controller: ResourceController<Box<dyn Sound>>,
    pub(crate) sub_track_controller: ResourceController<Track>,
    pub(crate) internal_buffer_size: usize,
}

impl TrackHandle {
    /// Plays a sound.
    pub(crate) fn play<D: SoundData>(&mut self, sound_data: D) -> Result<D::Handle, PlaySoundError<D::Error>> {
        let (sound, handle) = sound_data
            .into_sound(self.backend_sample_rate)
            .map_err(PlaySoundError::IntoSoundError)?;
        self.sound_controller.insert(sound).map_err(|_| PlaySoundError::SoundLimitReached)?;
        Ok(handle)
    }

    /// Adds a spatial child track to this track.
    pub(crate) fn add_spatial_sub_track(
        &mut self,
        position: Point3<f32>,
        builder: SpatialTrackBuilder,
    ) -> Result<SpatialTrackHandle, ResourceLimitReached> {
        let (track, handle) = builder.build(self.renderer_shared.clone(), self.internal_buffer_size, position);
        self.sub_track_controller.insert(track)?;
        Ok(handle)
    }

    /// Sets the (post-effects) volume of the mixer track.
    pub(crate) fn set_volume(&mut self, volume: Decibels, tween_duration: Duration) {
        self.command_writers.set_volume.write(ValueChangeCommand {
            target: volume,
            tween_duration,
        })
    }
}

impl Drop for TrackHandle {
    fn drop(&mut self) {
        self.shared.mark_for_removal();
    }
}
