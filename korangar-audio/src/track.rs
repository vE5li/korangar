//! Organizes audio.
//!
//! the audio engine has an internal mixer which works like a real-life mixing
//! console. Sounds can be played on "tracks", which are individual streams of
//! audio.
//!
//! Tracks can also be spatialized, which gives them a position in a 3D space
//! relative to a [listener](crate::listener).

mod main;
mod sub;

use std::sync::atomic::{AtomicBool, Ordering};

pub(crate) use main::{MainTrack, MainTrackBuilder, MainTrackHandle};
pub(crate) use sub::{SpatialTrackBuilder, SpatialTrackDistances, SpatialTrackHandle, Track, TrackBuilder, TrackHandle};

pub(crate) struct TrackShared {
    removed: AtomicBool,
}

impl TrackShared {
    pub(crate) fn new() -> Self {
        Self {
            removed: AtomicBool::new(false),
        }
    }

    #[must_use]
    pub(crate) fn is_marked_for_removal(&self) -> bool {
        self.removed.load(Ordering::SeqCst)
    }

    pub(crate) fn mark_for_removal(&self) {
        self.removed.store(true, Ordering::SeqCst);
    }
}
