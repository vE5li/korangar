mod stream_manager;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;

use self::stream_manager::StreamManager;
use super::{Error, OutputDevice};
use crate::backend::{Backend, Renderer};
use crate::device_info::{DeviceId, DeviceInfo, OutputDevicePreference};

const CHECK_STREAM_INTERVAL: Duration = Duration::from_millis(500);

enum State {
    Empty,
    Uninitialized {
        output: OutputDevice,
        preference: Arc<OutputDevicePreference>,
    },
    Initialized {
        should_drop: Arc<AtomicBool>,
    },
}

/// A backend that uses [cpal](https://crates.io/crates/cpal) to
/// connect a [`Renderer`] to the operating system's audio driver.
pub(crate) struct CpalBackend {
    state: State,
}

impl Backend for CpalBackend {
    type Error = Error;

    fn setup(preferred: Option<DeviceId>) -> Result<(Self, DeviceInfo, Arc<OutputDevicePreference>), Self::Error> {
        let output = OutputDevice::resolve(preferred.as_ref())?;
        let device_info = output.device_info();
        let available = OutputDevice::list_all();
        let preference = Arc::new(OutputDevicePreference::new(preferred, available));
        Ok((
            Self {
                state: State::Uninitialized { output, preference: preference.clone() },
            },
            device_info,
            preference,
        ))
    }

    fn start(&mut self, renderer: Renderer, current_sample_rate: Arc<AtomicU32>) -> Result<(), Self::Error> {
        let state = std::mem::replace(&mut self.state, State::Empty);
        let State::Uninitialized { output, preference } = state else {
            panic!("cannot initialize the audio backend multiple times");
        };

        let should_drop = Arc::new(AtomicBool::new(false));
        let should_drop_clone = should_drop.clone();

        let (mut initial_result_producer, mut initial_result_consumer) =
            rtrb::RingBuffer::new(1);

        // Monitoring thread: polls for device changes and stream errors.
        // Wakes immediately when the user changes the preferred device,
        // or every CHECK_STREAM_INTERVAL to catch system-level changes.
        std::thread::spawn(move || {
            let mut manager = StreamManager::new(renderer);
            let mut current_device = output;

            let mut error_consumer = match manager.start_stream(&current_device) {
                Ok(consumer) => {
                    initial_result_producer.push(Ok(())).unwrap();
                    consumer
                }
                Err(err) => {
                    initial_result_producer.push(Err(err)).unwrap();
                    return;
                }
            };

            loop {
                preference.wait_for_change(CHECK_STREAM_INTERVAL);
                if should_drop.load(Ordering::SeqCst) {
                    break;
                }

                let needs_restart = manager.has_stream_error(&mut error_consumer);

                if let Ok(target) = OutputDevice::resolve(preference.get().as_ref()) {
                    if needs_restart || target.id() != current_device.id() {
                        manager.stop_stream();
                        if let Ok(consumer) = manager.start_stream(&target) {
                            current_sample_rate.store(target.config.sample_rate, Ordering::SeqCst);
                            current_device = target;
                            error_consumer = consumer;
                        }
                    }
                }

                // Refresh the available device list for the UI.
                preference.update_available_devices(OutputDevice::list_all());
            }
        });

        loop {
            if let Ok(result) = initial_result_consumer.pop() {
                result?;
                break;
            }
            std::thread::sleep(Duration::from_micros(100));
        }

        self.state = State::Initialized { should_drop: should_drop_clone };
        Ok(())
    }
}

impl Drop for CpalBackend {
    fn drop(&mut self) {
        if let State::Initialized { should_drop } = &self.state {
            should_drop.store(true, Ordering::SeqCst);
        }
    }
}
