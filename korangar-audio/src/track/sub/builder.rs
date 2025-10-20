use std::sync::Arc;

use super::{Track, TrackHandle, TrackShared, command_writers_and_readers};
use crate::backend::RendererShared;
use crate::backend::resources::ResourceStorage;
use crate::decibels::Decibels;
use crate::frame::Frame;
use crate::parameter::Parameter;
use crate::playback_state_manager::PlaybackStateManager;

/// Configures a mixer track.
#[derive(Default)]
pub(crate) struct TrackBuilder {}

impl TrackBuilder {
    #[must_use]
    pub(crate) fn build(self, renderer_shared: Arc<RendererShared>, internal_buffer_size: usize) -> (Track, TrackHandle) {
        let (command_writers, command_readers) = command_writers_and_readers();
        let shared = Arc::new(TrackShared::new());
        let (sounds, sound_controller) = ResourceStorage::new(128);
        let (sub_tracks, sub_track_controller) = ResourceStorage::new(128);
        let track = Track {
            shared: shared.clone(),
            command_readers,
            volume: Parameter::new(Decibels::IDENTITY),
            sounds,
            sub_tracks,
            persist_until_sounds_finish: false,
            spatial_data: None,
            playback_state_manager: PlaybackStateManager::new(),
            temp_buffer: vec![Frame::ZERO; internal_buffer_size],
        };
        let handle = TrackHandle {
            renderer_shared,
            shared,
            command_writers,
            sound_controller,
            sub_track_controller,
            internal_buffer_size,
        };
        (track, handle)
    }
}
