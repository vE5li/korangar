use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use rtrb::{Consumer, Producer, RingBuffer};

use super::{Shared, TimestampedFrame};
use crate::frame::Frame;
use crate::resampler::Resampler;
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
    source_sample_rate: u32,
    backend_sample_rate: u32,
    resampler: Option<Resampler>,
    /// Queue of resampled frames waiting to be pushed to the ring buffer.
    /// Each entry is (resampled_frame, source_frame_index).
    resampler_output_queue: VecDeque<(Frame, usize)>,
    /// Count of how many source frames we've pushed to the resampler
    source_frames_pushed: u64,
    /// Count of how many resampled frames we've output
    resampled_frames_output: u64,
}

impl DecodeScheduler {
    pub(crate) fn new(
        mut decoder: SymphoniaDecoder,
        settings: StreamingSoundSettings,
        shared: Arc<Shared>,
        backend_sample_rate: u32,
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
        let source_sample_rate = decoder.sample_rate();
        let decoder_current_frame_index = decoder.seek(0)?;

        let resampler = if source_sample_rate != backend_sample_rate {
            Some(Resampler::new(source_sample_rate, backend_sample_rate))
        } else {
            None
        };

        let scheduler = Self {
            decoder,
            num_frames,
            transport: Transport::new(settings.loops, num_frames),
            decoder_current_frame_index,
            decoded_chunk: None,
            frame_producer,
            shared,
            source_sample_rate,
            backend_sample_rate,
            resampler,
            resampler_output_queue: VecDeque::new(),
            source_frames_pushed: 0,
            resampled_frames_output: 0,
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

        if let Some(resampler) = &mut self.resampler {
            // First, try to push any queued resampled frames to the ring buffer.
            while !self.frame_producer.is_full() && !self.resampler_output_queue.is_empty() {
                let (frame, index) = self.resampler_output_queue.pop_front().unwrap();

                self.frame_producer
                    .push(TimestampedFrame { frame, index })
                    .expect("frame producer should not be full");

                self.resampled_frames_output += 1;
            }

            if self.frame_producer.is_full() {
                return Ok(NextStep::Wait);
            }

            // Get resampled frames from the resampler's output buffer and push to ring
            // buffer.
            while !self.frame_producer.is_full() && resampler.has_output() {
                let resampled_frame = resampler.get_frame();

                // Calculate which source frame this output corresponds to
                // Based on the ratio of frames processed.
                let source_index =
                    ((self.resampled_frames_output * self.source_sample_rate as u64) / self.backend_sample_rate as u64) as usize;

                self.frame_producer
                    .push(TimestampedFrame {
                        frame: resampled_frame,
                        index: source_index,
                    })
                    .expect("frame producer should not be full");

                self.resampled_frames_output += 1;
            }

            // If ring buffer is full and resampler still has output, queue remaining
            // frames.
            while resampler.has_output() {
                let resampled_frame = resampler.get_frame();
                let source_index =
                    ((self.resampled_frames_output * self.source_sample_rate as u64) / self.backend_sample_rate as u64) as usize;
                self.resampler_output_queue.push_back((resampled_frame, source_index));
                self.resampled_frames_output += 1;
            }

            if !self.resampler_output_queue.is_empty() {
                return Ok(NextStep::Wait);
            }

            // If transport is still playing, push a new source frame to the resampler.
            if self.transport.playing {
                let source_frame = self.frame_at_index(self.transport.position)?;
                let resampler = self.resampler.as_mut().unwrap();
                resampler.push_frame(source_frame);
                self.source_frames_pushed += 1;
                self.transport.increment_position(self.num_frames);

                if !self.transport.playing {
                    self.shared.reached_end.store(true, Ordering::SeqCst);
                    // Don't end immediately - we may still have resampled frames to output.
                    return Ok(NextStep::Continue);
                }
            } else {
                // Transport stopped and no more output - end the thread.
                let resampler = self.resampler.as_ref().unwrap();
                if !resampler.has_output() && self.resampler_output_queue.is_empty() {
                    self.shared.reached_end.store(true, Ordering::SeqCst);
                    return Ok(NextStep::End);
                }
            }

            Ok(NextStep::Continue)
        } else {
            // No resampling needed
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
