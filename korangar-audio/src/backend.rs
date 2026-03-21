use std::sync::Arc;

pub(crate) mod cpal;
mod renderer;
pub(crate) mod resources;

pub(crate) use renderer::{MIXER_SAMPLE_RATE, Renderer};
use crate::device_info::{DeviceId, DeviceInfo, OutputDevicePreference};

pub(crate) type DefaultBackend = cpal::CpalBackend;

/// Connects a [`Renderer`] to a platform audio API.
pub(crate) trait Backend: Sized {
    type Error;

    /// Queries the platform for a suitable audio device.
    fn setup(preferred: Option<DeviceId>) -> Result<(Self, DeviceInfo, Arc<OutputDevicePreference>), Self::Error>;

    /// Starts audio playback with the given renderer.
    fn start(&mut self, renderer: Renderer) -> Result<(), Self::Error>;
}
