use super::resources::Resources;
use crate::Frame;

/// Produces [`Frame`]s of audio data to be consumed by a
/// low-level audio API.
/// 
/// Walks the mixer tree (main track → sub-tracks → sounds) each callback,
/// mixing all active audio into the output buffer.
pub(crate) struct Renderer {
    /// Time step per sample, derived from the device sample rate.
    dt: f64,
    resources: Resources,
    internal_buffer_size: usize,
    temp_buffer: Vec<Frame>,
}

impl Renderer {
    #[must_use]
    pub(crate) fn new(sample_rate: u32, internal_buffer_size: usize, resources: Resources) -> Self {
        Self {
            dt: 1.0 / sample_rate as f64,
            resources,
            internal_buffer_size,
            temp_buffer: vec![Frame::ZERO; internal_buffer_size],
        }
    }

    /// Called when the audio device's sample rate changes.
    pub(crate) fn on_change_sample_rate(&mut self, sample_rate: u32) {
        self.dt = 1.0 / sample_rate as f64;
    }

    /// Called by the backend when it's time to process
    /// a new batch of samples.
    pub(crate) fn on_start_processing(&mut self) {
        self.resources.mixer.on_start_processing();
        self.resources.listener.on_start_processing();
    }

    /// Produces the next [`Frame`]s of audio.
    pub(crate) fn process(&mut self, out: &mut [f32], num_channels: u16) {
        for chunk in out.chunks_mut(self.internal_buffer_size * num_channels as usize) {
            self.process_chunk(chunk, num_channels);
        }
    }

    fn process_chunk(&mut self, chunk: &mut [f32], num_channels: u16) {
        let num_frames = chunk.len() / num_channels as usize;

        self.resources.listener.update(self.dt * num_frames as f64);

        self.resources
            .mixer
            .process(&mut self.temp_buffer[..num_frames], self.dt, &self.resources.listener);

        // Convert from frames to requested number of channels.
        for (i, channels) in chunk.chunks_mut(num_channels.into()).enumerate() {
            let mut frame = self.temp_buffer[i];
            frame.left = frame.left.clamp(-1.0, 1.0);
            frame.right = frame.right.clamp(-1.0, 1.0);
            if num_channels == 1 {
                channels[0] = (frame.left + frame.right) / 2.0;
            } else {
                channels[0] = frame.left;
                channels[1] = frame.right;
                // If there's more channels, send silence to them. If we don't,
                // we might get bad sounds outputted to those channels.
                for channel in channels.iter_mut().skip(2) {
                    *channel = 0.0;
                }
            }
        }
        self.temp_buffer.fill(Frame::ZERO);
    }
}
