//! Plays audio using [cpal](https://crates.io/crates/cpal).

mod error;
use cpal::{Device, StreamConfig};
pub(crate) use error::Error;

/// Settings for the cpal kira.
#[derive(Clone, Default)]
pub(crate) struct CpalBackendSettings {
    /// The output audio device to use. If [`None`], the default output
    /// device will be used.
    pub(crate) device: Option<Device>,
    /// A StreamConfig given by Cpal. If [`None`], the default supported
    /// config will be used. You can also get a supported config of your
    /// choosing using Cpal functions.
    pub(crate) config: Option<StreamConfig>,
}

#[cfg(target_arch = "wasm32")]
mod wasm;
#[cfg(target_arch = "wasm32")]
pub(crate) use wasm::CpalBackend;

#[cfg(not(target_arch = "wasm32"))]
mod desktop;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use desktop::CpalBackend;
