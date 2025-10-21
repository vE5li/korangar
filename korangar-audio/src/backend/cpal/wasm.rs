use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Stream, StreamConfig};
use send_wrapper::SendWrapper;

use super::{Error, default_device_and_config};
use crate::backend::{Backend, Renderer};

enum State {
    Empty,
    Uninitialized { device: Device, config: StreamConfig },
    Initialized { _stream: Stream },
}

/// A kira that uses [cpal](https://crates.io/crates/cpal) to
/// connect a [`Renderer`] to the operating system's audio driver.
pub(crate) struct CpalBackend {
    state: SendWrapper<State>,
}

impl Backend for CpalBackend {
    type Error = Error;

    fn setup(_internal_buffer_size: usize) -> Result<(Self, u32), Self::Error> {
        let (device, config) = default_device_and_config()?;
        let sample_rate = config.sample_rate.0;

        Ok((
            Self {
                state: SendWrapper::new(State::Uninitialized { device, config }),
            },
            sample_rate,
        ))
    }

    fn start(&mut self, mut renderer: Renderer) -> Result<(), Self::Error> {
        if let State::Uninitialized { device, config } = std::mem::replace(&mut *self.state, State::Empty) {
            let channels = config.channels;
            let stream = device.build_output_stream(
                &config,
                move |data: &mut [f32], _| {
                    renderer.on_start_processing();
                    renderer.process(data, channels);
                },
                move |_| {},
                None,
            )?;
            stream.play()?;
            self.state = SendWrapper::new(State::Initialized { _stream: stream });
        } else {
            panic!("cannot initialize the kira multiple times")
        }
        Ok(())
    }
}
