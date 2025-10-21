use std::sync::Arc;

use cgmath::Point3;

use super::{SpatialData, SpatialTrackHandle, Track, TrackShared, command_writers_and_readers};
use crate::backend::RendererShared;
use crate::backend::resources::ResourceStorage;
use crate::decibels::Decibels;
use crate::frame::Frame;
use crate::parameter::Parameter;
use crate::playback_state_manager::PlaybackStateManager;

/// Configures a spatial mixer track.
pub(crate) struct SpatialTrackBuilder {
    pub(crate) persist_until_sounds_finish: bool,
    /// The distances from a listener at which the track is loudest and
    /// quietest.
    pub(crate) distances: SpatialTrackDistances,
    /// How the track's volume will change with distance.
    ///
    /// If false, the track will output at a constant volume.
    pub(crate) use_linear_attenuation_function: bool,
}

impl SpatialTrackBuilder {
    /// Creates a new [`SpatialTrackBuilder`] with the default settings.
    #[must_use]
    pub(crate) fn new() -> Self {
        Self {
            persist_until_sounds_finish: false,
            distances: SpatialTrackDistances::default(),
            use_linear_attenuation_function: true,
        }
    }

    /// Sets whether the track should stay alive while sounds are playing on it.
    ///
    /// By default, as soon as a track's handle is dropped, the track is
    /// unloaded. If this is set to `true`, the track will wait until all
    /// sounds on the track are finished before unloading.
    pub(crate) fn persist_until_sounds_finish(self, persist: bool) -> Self {
        Self {
            persist_until_sounds_finish: persist,
            ..self
        }
    }

    /// Sets the distances from a listener at which the emitter is loudest and
    /// quietest.
    #[must_use]
    pub(crate) fn distances(self, distances: impl Into<SpatialTrackDistances>) -> Self {
        Self {
            distances: distances.into(),
            ..self
        }
    }

    /// Sets how the emitter's volume will change with distance.
    ///
    /// If `false`, the emitter will output at a constant volume.
    #[must_use]
    pub(crate) fn use_linear_attenuation_function(self, use_linear_attenuation_function: bool) -> Self {
        Self {
            use_linear_attenuation_function,
            ..self
        }
    }

    #[must_use]
    pub(crate) fn build(
        self,
        renderer_shared: Arc<RendererShared>,
        internal_buffer_size: usize,
        position: Point3<f32>,
    ) -> (Track, SpatialTrackHandle) {
        let backend_sample_rate = renderer_shared.sample_rate.load(std::sync::atomic::Ordering::SeqCst);
        let (_command_writers, command_readers) = command_writers_and_readers();
        let shared = Arc::new(TrackShared::new());
        let (sounds, sound_controller) = ResourceStorage::new(128);
        let (sub_tracks, _sub_track_controller) = ResourceStorage::new(128);
        let track = Track {
            shared: shared.clone(),
            command_readers,
            volume: Parameter::new(Decibels::IDENTITY),
            sounds,
            sub_tracks,
            persist_until_sounds_finish: self.persist_until_sounds_finish,
            spatial_data: Some(SpatialData {
                position: Parameter::new(position),
                distances: self.distances,
                use_linear_attenuation_function: self.use_linear_attenuation_function,
                spatialization_strength: Parameter::new(0.75),
            }),
            playback_state_manager: PlaybackStateManager::new(),
            temp_buffer: vec![Frame::ZERO; internal_buffer_size],
        };
        let handle = SpatialTrackHandle {
            backend_sample_rate,
            shared,
            sound_controller,
        };
        (track, handle)
    }
}

impl Default for SpatialTrackBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// The distances from a listener at which an emitter is loudest and quietest.
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct SpatialTrackDistances {
    /// The distance from a listener at which an emitter outputs at full volume.
    pub(crate) min_distance: f32,
    /// The distance from a listener at which an emitter becomes inaudible.
    pub(crate) max_distance: f32,
}

impl SpatialTrackDistances {
    #[must_use]
    pub(crate) fn relative_distance(&self, distance: f32) -> f32 {
        let distance = distance.clamp(self.min_distance, self.max_distance);
        (distance - self.min_distance) / (self.max_distance - self.min_distance)
    }
}

impl Default for SpatialTrackDistances {
    fn default() -> Self {
        Self {
            min_distance: 1.0,
            max_distance: 100.0,
        }
    }
}
