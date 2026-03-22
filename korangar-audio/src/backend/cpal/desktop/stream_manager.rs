use std::ops::{Deref, DerefMut};

use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{Stream, StreamError};
use rtrb::{Consumer, Producer, RingBuffer};

use super::super::{Error, OutputDevice};
use crate::backend::Renderer;

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

/// Manages a single cpal audio stream.
pub(super) struct StreamManager {
    state: State,
}

impl StreamManager {
    /// Creates a new stream manager in an idle state.
    pub(super) fn new(renderer: Renderer) -> Self {
        Self {
            state: State::Idle { renderer },
        }
    }

    /// Returns `true` if there are any stream errors that require a restart
    /// (e.g. device disconnection).
    pub(super) fn has_stream_error(&self, error_consumer: &mut Consumer<StreamError>) -> bool {
        let mut needs_restart = false;
        while let Ok(error) = error_consumer.pop() {
            match error {
                StreamError::DeviceNotAvailable => needs_restart = true,
                StreamError::StreamInvalidated | StreamError::BufferUnderrun | StreamError::BackendSpecific { err: _ } => {}
            }
        }
        needs_restart
    }

    /// Starts the stream on the given output device. Updates the renderer's
    /// sample rate. Returns a consumer for stream errors.
    pub(super) fn start_stream(
        &mut self,
        output: &OutputDevice,
    ) -> Result<Consumer<StreamError>, Error> {
        // Take the idle renderer, or panic if the stream is already running.
        let State::Idle { mut renderer } = std::mem::replace(&mut self.state, State::Empty) else {
            panic!("trying to start a stream when the stream manager is not idle");
        };
        renderer.on_change_sample_rate(output.config.sample_rate);
        let (mut renderer_wrapper, renderer_consumer) = SendOnDrop::new(renderer);
        let (mut unhandled_stream_error_producer, unhandled_stream_error_consumer) = RingBuffer::new(64);
        let channels = output.config.channels;
        let stream = output.device.build_output_stream(
            &output.config,
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

    /// Stops the current stream, returning the stream manager to idle state.
    pub(super) fn stop_stream(&mut self) {
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
