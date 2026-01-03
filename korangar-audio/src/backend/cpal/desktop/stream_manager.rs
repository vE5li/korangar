use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{BufferSize, Device, Stream, StreamConfig, StreamError};
use rtrb::{Consumer, Producer, RingBuffer};

use super::super::{Error, default_device_and_config};
use crate::backend::Renderer;

const CHECK_STREAM_INTERVAL: Duration = Duration::from_millis(500);

#[allow(clippy::large_enum_variant)]
enum State {
    Empty,
    Idle {
        renderer: Renderer,
    },
    Running {
        stream: Stream,
        renderer_consumer: Consumer<Renderer>,
    },
}

pub(super) struct StreamManagerController {
    should_drop: Arc<AtomicBool>,
}

impl StreamManagerController {
    pub(crate) fn stop(&self) {
        self.should_drop.store(true, Ordering::SeqCst);
    }
}

/// Starts a cpal stream and restarts it if needed in the case of device changes
/// or disconnections.
pub(super) struct StreamManager {
    state: State,
    device_name: String,
    sample_rate: u32,
    buffer_size: BufferSize,
}

impl StreamManager {
    pub(crate) fn start(
        renderer: Renderer,
        device: Device,
        mut config: StreamConfig,
        buffer_size: BufferSize,
    ) -> Result<StreamManagerController, Error> {
        let should_drop = Arc::new(AtomicBool::new(false));
        let should_drop_clone = should_drop.clone();

        let (mut initial_result_producer, mut initial_result_consumer) = RingBuffer::new(1);

        std::thread::spawn(move || {
            let mut stream_manager = StreamManager {
                state: State::Idle { renderer },
                device_name: device_name(&device),
                sample_rate: config.sample_rate,
                buffer_size,
            };
            let mut unhandled_stream_error_consumer = match stream_manager.start_stream(&device, &mut config) {
                Ok(unhandled_stream_error_consumer) => {
                    initial_result_producer.push(Ok(())).unwrap();
                    unhandled_stream_error_consumer
                }
                Err(err) => {
                    initial_result_producer.push(Err(err)).unwrap();
                    return;
                }
            };
            loop {
                std::thread::sleep(CHECK_STREAM_INTERVAL);
                if should_drop.load(Ordering::SeqCst) {
                    break;
                }
                stream_manager.check_stream(&mut unhandled_stream_error_consumer);
            }
        });

        loop {
            if let Ok(result) = initial_result_consumer.pop() {
                result?;
                break;
            }
            std::thread::sleep(Duration::from_micros(100));
        }

        Ok(StreamManagerController {
            should_drop: should_drop_clone,
        })
    }

    /// Restarts the stream if the audio device gets disconnected.
    fn check_stream(&mut self, unhandled_stream_error_consumer: &mut Consumer<StreamError>) {
        if let State::Running { .. } = &self.state {
            while let Ok(error) = unhandled_stream_error_consumer.pop() {
                match error {
                    // Check for device disconnection.
                    StreamError::DeviceNotAvailable => {
                        self.stop_stream();
                        if let Ok((device, mut config)) = default_device_and_config() {
                            *unhandled_stream_error_consumer = self.start_stream(&device, &mut config).unwrap();
                        }
                    }
                    StreamError::StreamInvalidated | StreamError::BufferUnderrun | StreamError::BackendSpecific { err: _ } => {}
                }
            }
        }
    }

    fn start_stream(&mut self, device: &Device, config: &mut StreamConfig) -> Result<Consumer<StreamError>, Error> {
        let mut renderer = if let State::Idle { renderer } = std::mem::replace(&mut self.state, State::Empty) {
            renderer
        } else {
            panic!("trying to start a stream when the stream manager is not idle");
        };
        config.buffer_size = self.buffer_size;
        let device_name = device_name(device);
        let sample_rate = config.sample_rate;
        if sample_rate != self.sample_rate {
            renderer.on_change_sample_rate(sample_rate);
        }
        self.device_name = device_name;
        self.sample_rate = sample_rate;
        let (mut renderer_wrapper, renderer_consumer) = SendOnDrop::new(renderer);
        let (mut unhandled_stream_error_producer, unhandled_stream_error_consumer) = RingBuffer::new(64);
        let channels = config.channels;
        let stream = device.build_output_stream(
            config,
            move |data: &mut [f32], _| {
                process_renderer(&mut renderer_wrapper, data, channels);
            },
            move |error| {
                let _ = unhandled_stream_error_producer.push(error);
            },
            None,
        )?;
        stream.play()?;
        self.state = State::Running { stream, renderer_consumer };
        Ok(unhandled_stream_error_consumer)
    }

    fn stop_stream(&mut self) {
        if let State::Running {
            mut renderer_consumer,
            stream,
            ..
        } = std::mem::replace(&mut self.state, State::Empty)
        {
            drop(stream);
            let renderer = renderer_consumer
                .pop()
                .expect("could not retrieve the renderer after dropping a stream");
            self.state = State::Idle { renderer };
        } else {
            panic!("trying to stop the stream when it's not running")
        }
    }
}

fn device_name(device: &Device) -> String {
    device
        .description()
        .map(|description| description.name().to_string())
        .unwrap_or_else(|_| "device name unavailable".to_string())
}
fn process_renderer(renderer: &mut SendOnDrop<Renderer>, data: &mut [f32], channels: u16) {
    renderer.on_start_processing();
    renderer.process(data, channels);
}

/// Wraps `T` so that when it's dropped, it gets sent
/// back through a thread channel.
///
/// This allows us to retrieve the data after a closure
/// that takes ownership of the data is dropped because of,
/// for instance, a cpal error.
struct SendOnDrop<T> {
    data: Option<T>,
    producer: Producer<T>,
}

impl<T> SendOnDrop<T> {
    fn new(data: T) -> (Self, Consumer<T>) {
        let (producer, consumer) = RingBuffer::new(1);
        (
            Self {
                data: Some(data),
                producer,
            },
            consumer,
        )
    }
}

impl<T> Deref for SendOnDrop<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data.as_ref().unwrap()
    }
}

impl<T> DerefMut for SendOnDrop<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data.as_mut().unwrap()
    }
}

impl<T> Drop for SendOnDrop<T> {
    fn drop(&mut self) {
        self.producer.push(self.data.take().unwrap()).expect("send on drop producer full");
    }
}
