mod stream_manager;

use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{BufferSize, Device, StreamConfig};
use stream_manager::{StreamManager, StreamManagerController};

use super::{CpalBackendSettings, Error};
use crate::backend::{Backend, Renderer};

enum State {
    Empty,
    Uninitialized {
        device: Device,
        config: StreamConfig,
    },
    Initialized {
        stream_manager_controller: StreamManagerController,
    },
}

/// A backend that uses [cpal](https://crates.io/crates/cpal) to
/// connect a [`Renderer`] to the operating system's audio driver.
pub(crate) struct CpalBackend {
    state: State,
    /// Whether the device was specified by the user.
    custom_device: bool,
    buffer_size: BufferSize,
}

impl Backend for CpalBackend {
    type Error = Error;
    type Settings = CpalBackendSettings;

    fn setup(settings: Self::Settings, _internal_buffer_size: usize) -> Result<(Self, u32), Self::Error> {
        let host = cpal::default_host();

        let (device, custom_device) = match settings.device {
            Some(device) => (device, true),
            None => (host.default_output_device().ok_or(Error::NoDefaultOutputDevice)?, false),
        };

        let config = match settings.config {
            Some(config) => config,
            None => device.default_output_config()?.config(),
        };

        let sample_rate = config.sample_rate.0;
        let buffer_size = config.buffer_size;

        Ok((
            Self {
                state: State::Uninitialized { device, config },
                custom_device,
                buffer_size,
            },
            sample_rate,
        ))
    }

    fn start(&mut self, renderer: Renderer) -> Result<(), Self::Error> {
        let state = std::mem::replace(&mut self.state, State::Empty);
        if let State::Uninitialized { device, config } = state {
            self.state = State::Initialized {
                stream_manager_controller: StreamManager::start(renderer, device, config, self.custom_device, self.buffer_size)?,
            };
        } else {
            panic!("cannot initialize the kira multiple times")
        }
        Ok(())
    }
}

impl Drop for CpalBackend {
    fn drop(&mut self) {
        if let State::Initialized { stream_manager_controller } = &self.state {
            stream_manager_controller.stop();
        }
    }
}
