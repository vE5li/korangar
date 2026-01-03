//! Plays audio using [cpal](https://crates.io/crates/cpal).

mod error;

use cpal::traits::HostTrait;
use cpal::{BufferSize, Device, StreamConfig};
pub(crate) use error::Error;

#[cfg(target_arch = "wasm32")]
mod wasm;
#[cfg(target_arch = "wasm32")]
pub(crate) use wasm::CpalBackend;

#[cfg(not(target_arch = "wasm32"))]
mod desktop;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use desktop::CpalBackend;

pub(crate) fn default_device_and_config() -> Result<(Device, StreamConfig), Error> {
    let host = cpal::default_host();
    let device = host.default_output_device().ok_or(Error::NoDefaultOutputDevice)?;
    // We don't use the default sampling rate, since if the audio device switches,
    // we need to use the same configuration for it, or else the re-sampled audio
    // files won't play correctly (we re-sample audio files on load, not at
    // playtime). Stereo with 48 kHz should be supported by any device and is the
    // standard for many operating systems.
    let config = StreamConfig {
        channels: 2,
        sample_rate: 48000,
        buffer_size: BufferSize::Fixed(1200),
    };
    Ok((device, config))
}
