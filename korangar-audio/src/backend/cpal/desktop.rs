mod stream_manager;

use cpal::{BufferSize, Device, StreamConfig};

use self::stream_manager::{StreamManager, StreamManagerController};
use super::{Error, default_device_and_config};
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
    buffer_size: BufferSize,
}

impl Backend for CpalBackend {
    type Error = Error;

    fn setup(_internal_buffer_size: usize) -> Result<(Self, u32), Self::Error> {
        let (device, config) = default_device_and_config()?;
        let sample_rate = config.sample_rate;
        let buffer_size = config.buffer_size;

        Ok((
            Self {
                state: State::Uninitialized { device, config },
                buffer_size,
            },
            sample_rate,
        ))
    }

    fn start(&mut self, renderer: Renderer) -> Result<(), Self::Error> {
        let state = std::mem::replace(&mut self.state, State::Empty);
        if let State::Uninitialized { device, config } = state {
            self.state = State::Initialized {
                stream_manager_controller: StreamManager::start(renderer, device, config, self.buffer_size)?,
            };
        } else {
            panic!("cannot initialize the audio backend multiple times")
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
