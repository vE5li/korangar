use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use rtrb::{Consumer, Producer, RingBuffer};

use super::{Shared, TimestampedFrame};
use crate::frame::Frame;
use crate::sound::PlaybackState;
use crate::sound::error::FromFileError;
use crate::sound::streaming::{StreamingSoundSettings, SymphoniaDecoder};
use crate::sound::transport::Transport;

const BUFFER_SIZE: usize = 16_384;
const DECODER_THREAD_SLEEP_DURATION: Duration = Duration::from_millis(1);

pub(crate) enum NextStep {
    Continue,
    Wait,
    End,
}

pub(crate) struct DecodeScheduler {
    decoder: SymphoniaDecoder,
    num_frames: usize,
    transport: Transport,
    decoder_current_frame_index: usize,
    decoded_chunk: Option<DecodedChunk>,
    frame_producer: Producer<TimestampedFrame>,
    shared: Arc<Shared>,
}

impl DecodeScheduler {
    pub(crate) fn new(
        mut decoder: SymphoniaDecoder,
        settings: StreamingSoundSettings,
        shared: Arc<Shared>,
    ) -> Result<(Self, Consumer<TimestampedFrame>), FromFileError> {
        let (mut frame_producer, frame_consumer) = RingBuffer::new(BUFFER_SIZE);
        // Pre-seed the frame ring buffer with a zero frame. This is the "previous"
        // frame when the sound just started.
        frame_producer
            .push(TimestampedFrame {
                frame: Frame::ZERO,
                index: 0,
            })
            .expect("the frame producer shouldn't be full because we just created it");
        let num_frames = decoder.num_frames();
        let decoder_current_frame_index = decoder.seek(0)?;
        let scheduler = Self {
            decoder,
            num_frames,
            transport: Transport::new(settings.loops, num_frames),
            decoder_current_frame_index,
            decoded_chunk: None,
            frame_producer,
            shared,
        };
        Ok((scheduler, frame_consumer))
    }

    #[must_use]
    pub(crate) fn current_frame(&self) -> usize {
        self.transport.position
    }

    pub(crate) fn start(mut self) {
        std::thread::spawn(move || {
            loop {
                match self.run() {
                    Ok(result) => match result {
                        NextStep::Continue => {}
                        NextStep::Wait => std::thread::sleep(DECODER_THREAD_SLEEP_DURATION),
                        NextStep::End => break,
                    },
                    Err(error) => {
                        if let Some(loop_start) = self.should_restart_on_eof(&error) {
                            if self.seek_to_index(loop_start).is_err() {
                                self.shared.encountered_error.store(true, Ordering::SeqCst);
                                break;
                            }
                        } else {
                            self.shared.encountered_error.store(true, Ordering::SeqCst);
                            break;
                        }
                    }
                }
            }
        });
    }

    pub(crate) fn run(&mut self) -> Result<NextStep, FromFileError> {
        // If the sound was manually stopped, end the thread.
        if self.shared.state() == PlaybackState::Stopped {
            return Ok(NextStep::End);
        }
        // If the frame ring-buffer is full, sleep for a bit.
        if self.frame_producer.is_full() {
            return Ok(NextStep::Wait);
        }
        let frame = self.frame_at_index(self.transport.position)?;
        self.frame_producer
            .push(TimestampedFrame {
                frame,
                index: self.transport.position,
            })
            .expect("could not push frame to frame producer");
        self.transport.increment_position(self.num_frames);
        if !self.transport.playing {
            self.shared.reached_end.store(true, Ordering::SeqCst);
            return Ok(NextStep::End);
        }
        Ok(NextStep::Continue)
    }

    fn frame_at_index(&mut self, index: usize) -> Result<Frame, FromFileError> {
        if index >= self.num_frames {
            return Ok(Frame::ZERO);
        }
        // If the requested frame is already loaded, return it.
        if let Some(chunk) = &self.decoded_chunk
            && let Some(frame) = chunk.frame_at_index(index)
        {
            return Ok(frame);
        }
        // Otherwise, seek to the requested index and decode chunks sequentially until
        // we get the frame we want. Just because we seek to an index does not mean the
        // next decoded chunk will have the frame we want (or any frame at all, for that
        // matter), so we may need to decode multiple chunks to get the frame we care
        // about.
        if index < self.decoder_current_frame_index {
            self.decoder_current_frame_index = self.decoder.seek(index)?;
        }
        loop {
            let decoded_chunk = DecodedChunk {
                start_index: self.decoder_current_frame_index,
                frames: self.decoder.decode()?,
            };
            self.decoder_current_frame_index += decoded_chunk.frames.len();
            self.decoded_chunk = Some(decoded_chunk);
            if let Some(chunk) = &self.decoded_chunk
                && let Some(frame) = chunk.frame_at_index(index)
            {
                return Ok(frame);
            }
        }
    }

    fn seek_to_index(&mut self, index: usize) -> Result<(), FromFileError> {
        self.transport.seek_to(index, self.num_frames);
        self.decoder_current_frame_index = self.decoder.seek(index)?;
        Ok(())
    }

    fn should_restart_on_eof(&self, error: &FromFileError) -> Option<usize> {
        match error {
            FromFileError::IoError(io_err) if io_err.kind() == std::io::ErrorKind::UnexpectedEof => {
                self.transport.loop_region.map(|(loop_start, _)| loop_start)
            }
            FromFileError::SymphoniaError(sym_err) => {
                if let symphonia::core::errors::Error::IoError(io_err) = sym_err
                    && io_err.kind() == std::io::ErrorKind::UnexpectedEof
                {
                    return self.transport.loop_region.map(|(loop_start, _)| loop_start);
                }
                None
            }
            _ => None,
        }
    }
}

struct DecodedChunk {
    pub(crate) start_index: usize,
    pub(crate) frames: Vec<Frame>,
}

impl DecodedChunk {
    fn frame_at_index(&self, index: usize) -> Option<Frame> {
        if index < self.start_index {
            return None;
        }
        self.frames.get(index - self.start_index).copied()
    }
}
