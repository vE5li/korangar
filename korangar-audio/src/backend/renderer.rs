#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, print_debug};

use bytemuck::cast_slice;
use resampler::{Attenuation, Latency, ResamplerFir, SampleRate};

use super::resources::Resources;
use crate::Frame;

/// Fixed internal mixer sample rate. All sounds are resampled to this
/// rate, and the final output is resampled to the device rate.
pub(crate) const MIXER_SAMPLE_RATE: u32 = 48000;

/// Size of the internal mixing buffer in frames.
const INTERNAL_BUFFER_SIZE: usize = 256;

/// FIR resampler for the renderer's output stage (48kHz → device rate).
/// Uses -60dB attenuation which is sufficient for game audio.
struct OutputResampler {
    resampler: ResamplerFir,
    output_f32: Vec<f32>,
}

impl OutputResampler {
    fn new(device_sample_rate: u32) -> Self {
        let input_rate = SampleRate::try_from(MIXER_SAMPLE_RATE).unwrap();
        let output_rate = SampleRate::try_from(device_sample_rate).unwrap_or(SampleRate::Hz48000);
        let resampler = ResamplerFir::new(2, input_rate, output_rate, Latency::default(), Attenuation::Db60);
        let max_output = resampler.buffer_size_output();
        Self {
            resampler,
            output_f32: vec![0.0; max_output],
        }
    }

    /// Feeds input frames and appends resampled output to `output`.
    fn process(&mut self, input: &[Frame], output: &mut Vec<Frame>) {
        let input_f32: &[f32] = cast_slice(input);
        let mut input_pos = 0;

        while input_pos < input_f32.len() {
            match self.resampler.resample(&input_f32[input_pos..], &mut self.output_f32) {
                Ok((consumed, produced)) => {
                    if consumed == 0 && produced == 0 {
                        break;
                    }
                    input_pos += consumed;
                    if produced > 0 {
                        let output_frames: &[Frame] = cast_slice(&self.output_f32[..produced]);
                        output.extend_from_slice(output_frames);
                    }
                }
                Err(_) => break,
            }
        }
    }
}

/// Produces [`Frame`]s of audio data to be consumed by a
/// low-level audio API.
///
/// The mixer always runs at [`MIXER_SAMPLE_RATE`]. A final resampling
/// step converts to the device's native sample rate.
pub(crate) struct Renderer {
    resources: Resources,
    temp_buffer: Vec<Frame>,
    /// FIR resampler from mixer rate to device rate. `None` if they match.
    resampler: Option<OutputResampler>,
    /// Resampled frames that didn't fit in the previous callback's output.
    overflow: Vec<Frame>,
    /// Persistent buffer for collecting resampled output.
    resampled_buffer: Vec<Frame>,
}

impl Renderer {
    #[must_use]
    pub(crate) fn new(resources: Resources) -> Self {
        Self {
            resources,
            temp_buffer: vec![Frame::ZERO; INTERNAL_BUFFER_SIZE],
            resampler: None,
            overflow: Vec::new(),
            resampled_buffer: Vec::new(),
        }
    }

    /// Called when the audio device's sample rate changes.
    pub(crate) fn on_change_sample_rate(&mut self, device_sample_rate: u32) {
        self.overflow.clear();
        self.resampler = if device_sample_rate != MIXER_SAMPLE_RATE {
            Some(OutputResampler::new(device_sample_rate))
        } else {
            None
        };

        #[cfg(feature = "debug")]
        if self.resampler.is_some() {
            print_debug!(
                "[{}] resampling {}Hz -> {}Hz",
                "audio".magenta(), MIXER_SAMPLE_RATE, device_sample_rate
            );
        } else {
            print_debug!(
                "[{}] no resampling needed ({}Hz)",
                "audio".magenta(), MIXER_SAMPLE_RATE
            );
        }
    }

    pub(crate) fn on_start_processing(&mut self) {
        self.resources.mixer.on_start_processing();
        self.resources.listener.on_start_processing();
    }

    /// Produces the next [`Frame`]s of audio.
    pub(crate) fn process(&mut self, out: &mut [f32], num_channels: u16) {
        if self.resampler.is_some() {
            self.process_resampled(out, num_channels);
        } else {
            // Fast path: mixer rate == device rate, no resampling needed.
            for chunk in out.chunks_mut(INTERNAL_BUFFER_SIZE * num_channels as usize) {
                self.process_chunk_direct(chunk, num_channels);
            }
        }
    }

    /// Process when resampling is needed: mix at 48kHz, then resample to device rate.
    fn process_resampled(&mut self, out: &mut [f32], num_channels: u16) {
        let device_frames_needed = out.len() / num_channels as usize;
        let dt = 1.0 / MIXER_SAMPLE_RATE as f64;

        // Start with any leftover frames from the previous callback.
        self.resampled_buffer.clear();
        self.resampled_buffer.append(&mut self.overflow);

        let resampler = self.resampler.as_mut().unwrap();

        // Generate mixer chunks and resample until we have enough output.
        while self.resampled_buffer.len() < device_frames_needed {
            let chunk_size = INTERNAL_BUFFER_SIZE;
            self.resources.listener.update(dt * chunk_size as f64);
            self.resources.mixer.process(
                &mut self.temp_buffer[..chunk_size],
                dt,
                &self.resources.listener,
            );

            resampler.process(&self.temp_buffer[..chunk_size], &mut self.resampled_buffer);
            self.temp_buffer[..chunk_size].fill(Frame::ZERO);
        }

        // Write exactly device_frames_needed to output.
        for (i, frame) in self.resampled_buffer[..device_frames_needed].iter().enumerate() {
            let mut f = *frame;
            f.left = f.left.clamp(-1.0, 1.0);
            f.right = f.right.clamp(-1.0, 1.0);
            let base = i * num_channels as usize;
            if num_channels == 1 {
                out[base] = (f.left + f.right) / 2.0;
            } else {
                out[base] = f.left;
                out[base + 1] = f.right;
                for ch in 2..num_channels as usize {
                    out[base + ch] = 0.0;
                }
            }
        }

        // Save any extra frames for the next callback.
        if self.resampled_buffer.len() > device_frames_needed {
            self.overflow.extend_from_slice(&self.resampled_buffer[device_frames_needed..]);
        }
    }

    /// Direct path: no resampling, mixer rate == device rate.
    fn process_chunk_direct(&mut self, chunk: &mut [f32], num_channels: u16) {
        let num_frames = chunk.len() / num_channels as usize;
        let dt = 1.0 / MIXER_SAMPLE_RATE as f64;

        self.resources.listener.update(dt * num_frames as f64);
        self.resources
            .mixer
            .process(&mut self.temp_buffer[..num_frames], dt, &self.resources.listener);

        for (i, channels) in chunk.chunks_mut(num_channels.into()).enumerate() {
            let mut frame = self.temp_buffer[i];
            frame.left = frame.left.clamp(-1.0, 1.0);
            frame.right = frame.right.clamp(-1.0, 1.0);
            if num_channels == 1 {
                channels[0] = (frame.left + frame.right) / 2.0;
            } else {
                channels[0] = frame.left;
                channels[1] = frame.right;
                for channel in channels.iter_mut().skip(2) {
                    *channel = 0.0;
                }
            }
        }
        self.temp_buffer.fill(Frame::ZERO);
    }
}
