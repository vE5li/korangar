//! Sources of audio.
//!
//! Any type that implements [`SoundData`] can be played using
//! [`AudioManager::play`](crate::AudioManager::play). the audio engine comes
//! with two [`SoundData`] implementations:
//!
//! - [`StaticSoundData`](static_sound::StaticSoundData), which loads an entire
//!   chunk of audio into memory. This is more appropriate for short sounds,
//!   sounds you want to play multiple times, or sounds where consistent start
//!   times are important.
//! - [`StreamingSoundData`](streaming::StreamingSoundData), which streams audio
//!   from a file or cursor (only available on desktop platforms). This is more
//!   appropriate for long sounds that you only play once at a time, like
//!   background music. Streaming sounds use less memory than static sounds.
//!
//! These two sound types should cover most use cases, but if you need something
//! else, you can create your own types that implement the [`SoundData`] and
//! [`Sound`] traits.

mod error;
pub(crate) mod static_sound;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod streaming;

mod symphonia;
mod transport;

use crate::frame::Frame;

/// A source of audio that is loaded, but not yet playing.
pub(crate) trait SoundData {
    /// Errors that can occur when starting the sound.
    type Error;

    /// The type that can be used to control the sound once
    /// it has started.
    type Handle;

    /// Converts the loaded sound into a live, playing sound
    /// and a handle to control it.
    ///
    /// The [`Sound`] implementation will be sent to the audio renderer
    /// for playback, and the handle will be returned to the user by
    /// [`AudioManager::play`](crate::AudioManager::play).
    #[allow(clippy::type_complexity)]
    fn into_sound(self) -> Result<(Box<dyn Sound>, Self::Handle), Self::Error>;
}

/// An actively playing sound.
///
/// For performance reasons, the methods of this trait should not allocate
/// or deallocate memory.
#[allow(unused_variables)]
pub(crate) trait Sound: Send {
    /// Called whenever a new batch of audio samples is requested by the
    /// kira.
    ///
    /// This is a good place to put code that needs to run fairly frequently,
    /// but not for every single audio sample.
    fn on_start_processing(&mut self) {}

    /// Produces the next [`Frame`]s of audio. This should overwrite
    /// the entire `out` slice with new audio.
    ///
    /// `dt` is the time between each frame (in seconds).
    fn process(&mut self, out: &mut [Frame], dt: f64);

    /// Returns `true` if the sound is finished and can be unloaded.
    ///
    /// For finite sounds, this will typically be when playback has reached the
    /// end of the sound. For infinite sounds, this will typically be when the
    /// handle for the sound is dropped.
    #[must_use]
    fn finished(&self) -> bool;
}

/// The playback state of a sound.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum PlaybackState {
    /// The sound is playing normally.
    Playing,
    /// The sound is fading out, and when the fade-out
    /// is finished, playback will stop.
    Stopping,
    /// The sound has stopped and can no longer be resumed.
    Stopped,
}

impl PlaybackState {
    /// Whether the sound is advancing and outputting audio given
    /// its current playback state.
    pub(crate) fn is_advancing(self) -> bool {
        match self {
            PlaybackState::Playing => true,
            PlaybackState::Stopping => true,
            PlaybackState::Stopped => false,
        }
    }
}
