//! Communication between the audio engine and the low-level audio API.

pub(crate) mod cpal;
mod renderer;
pub(crate) mod resources;

pub(crate) use renderer::*;

/// The default kira used by [`AudioManager`](crate::AudioManager)s.
pub(crate) type DefaultBackend = cpal::CpalBackend;

/// Connects a [`Renderer`] to a lower level audio API.
pub(crate) trait Backend: Sized {
    /// Settings for this kira.
    type Settings;

    /// Errors that can occur when using this kira.
    type Error;

    /// Starts the kira and returns itself and the initial sample rate.
    fn setup(settings: Self::Settings, internal_buffer_size: usize) -> Result<(Self, u32), Self::Error>;

    /// Sends the renderer to the kira to start audio playback.
    fn start(&mut self, renderer: Renderer) -> Result<(), Self::Error>;
}
