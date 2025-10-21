use std::sync::Arc;

use bytemuck::{cast_slice, cast_slice_mut};
use resampler::{ResamplerFft, SampleRate};

use crate::frame::Frame;

/// A wrapper hte FFT based resampler that adapts its chunk-based API
/// to a frame-by-frame interface for audio playback.
///
/// Since we're on the audio thread and control the calling pattern, we don't
/// need complex ring buffer logic - just simple chunked processing.
pub(crate) struct Resampler {
    resampler: ResamplerFft<2>,
    input_buffer: Vec<Frame>,
    input_length: usize,
    output_buffer: Vec<Frame>,
    output_position: usize,
    output_frames: usize,
    chunk_size_in: usize,
    chunk_size_out: usize,
}

impl Resampler {
    /// Creates a new resampler.
    ///
    /// # Parameters
    /// - `input_rate`: Sample rate of the input audio (e.g., 44100)
    /// - `output_rate`: Sample rate of the output audio (e.g., 48000)
    pub(crate) fn new(input_rate: u32, output_rate: u32) -> Self {
        let input_rate = SampleRate::try_from(input_rate).unwrap_or(SampleRate::Hz22050);
        let output_rate = SampleRate::try_from(output_rate).unwrap_or(SampleRate::Hz48000);

        let resampler = ResamplerFft::<2>::new(input_rate, output_rate);

        // ResamplerFft's chunk size is in f32 values, not frames.
        let chunk_size_in = resampler.chunk_size_input() / 2;
        let chunk_size_out = resampler.chunk_size_output() / 2;

        let input_buffer = vec![Frame::ZERO; chunk_size_in];
        let output_buffer = vec![Frame::ZERO; chunk_size_out];

        Self {
            resampler,
            input_buffer,
            input_length: 0,
            output_buffer,
            output_position: 0,
            output_frames: 0,
            chunk_size_in,
            chunk_size_out,
        }
    }

    /// Pushes a frame into the resampler. When enough frames are buffered,
    /// they will be automatically processed into the output buffer.
    pub(crate) fn push_frame(&mut self, frame: Frame) {
        self.input_buffer[self.input_length] = frame;
        self.input_length += 1;

        if self.input_length >= self.chunk_size_in {
            self.process_chunk();
        }
    }

    /// Returns the next resampled frame. Returns `Frame::ZERO` if no frames
    /// are available in the output buffer.
    pub(crate) fn get_frame(&mut self) -> Frame {
        if self.output_position < self.output_frames {
            let frame = self.output_buffer[self.output_position];
            self.output_position += 1;
            frame
        } else {
            Frame::ZERO
        }
    }

    /// Returns true if there are resampled frames available in the output
    /// buffer.
    pub(crate) fn has_output(&self) -> bool {
        self.output_position < self.output_frames
    }

    /// Processes the buffered input frames through the FFT based resampler and
    /// writes to the output buffer.
    fn process_chunk(&mut self) {
        let num_frames = self.input_length;
        if num_frames == 0 {
            return;
        }

        if num_frames < self.chunk_size_in {
            self.input_buffer[num_frames..self.chunk_size_in].fill(Frame::ZERO);
        }

        let input_f32_slice: &[f32] = cast_slice(&self.input_buffer[..self.chunk_size_in]);
        let output_f32_slice: &mut [f32] = cast_slice_mut(&mut self.output_buffer[..self.chunk_size_out]);

        self.resampler
            .resample(input_f32_slice, output_f32_slice)
            .expect("resampling failed");

        let actual_output_frames = if num_frames < self.chunk_size_in {
            // Only take the output corresponding to the real input (not the zero-padding).
            (self.chunk_size_out * num_frames) / self.chunk_size_in
        } else {
            self.chunk_size_out
        };

        self.output_position = 0;
        self.output_frames = actual_output_frames;
        self.input_length = 0;
    }

    /// Flushes any remaining buffered input frames through the resampler.
    /// Call this when the audio stream ends to ensure all frames are processed.
    #[allow(dead_code)]
    pub(crate) fn flush(&mut self) {
        if self.input_length > 0 {
            self.process_chunk();
        }
    }

    /// Resamples an entire buffer of frames at once.
    ///
    /// This is a batch processing method suitable for static sounds that are
    /// fully loaded into memory. For streaming audio, use the frame-by-frame
    /// API with `push_frame` and `get_frame`.
    ///
    /// # Parameters
    /// - `frames`: The input audio frames to resample
    ///
    /// # Returns
    /// A new buffer containing the resampled frames.
    pub(crate) fn resample_batch(&mut self, frames: &[Frame]) -> Arc<[Frame]> {
        let expected_output_frames = ((frames.len() as u64 * self.chunk_size_out as u64) / self.chunk_size_in as u64) as usize;
        let mut resampled_frames = vec![Frame::ZERO; expected_output_frames];

        let mut position = 0;
        let mut output_position = 0;

        while position < frames.len() {
            let remaining = frames.len() - position;
            let frames_to_copy = remaining.min(self.chunk_size_in);

            self.input_buffer[..frames_to_copy].copy_from_slice(&frames[position..position + frames_to_copy]);
            if frames_to_copy < self.chunk_size_in {
                self.input_buffer[frames_to_copy..].fill(Frame::ZERO);
            }

            let input_f32_slice: &[f32] = cast_slice(&self.input_buffer[..self.chunk_size_in]);
            let output_f32_slice: &mut [f32] = cast_slice_mut(&mut self.output_buffer[..self.chunk_size_out]);

            self.resampler
                .resample(input_f32_slice, output_f32_slice)
                .expect("resampling failed");

            if frames_to_copy < self.chunk_size_in {
                // Only take the output corresponding to the real input (not the zero-padding).
                let actual_output_frames = (self.chunk_size_out * frames_to_copy) / self.chunk_size_in;

                let frames_to_write = actual_output_frames.min(resampled_frames.len().saturating_sub(output_position));
                resampled_frames[output_position..output_position + frames_to_write]
                    .copy_from_slice(&self.output_buffer[..frames_to_write]);
                output_position += frames_to_write;
            } else {
                let frames_to_write = self.chunk_size_out.min(resampled_frames.len().saturating_sub(output_position));
                resampled_frames[output_position..output_position + frames_to_write]
                    .copy_from_slice(&self.output_buffer[..frames_to_write]);
                output_position += frames_to_write;
            }

            position += frames_to_copy;
        }

        resampled_frames.into()
    }
}
