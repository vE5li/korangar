use std::sync::Arc;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};

use super::data::StaticSoundData;
use super::{frame_at_index, num_frames};
use crate::decibels::Decibels;
use crate::frame::{Frame, interpolate_frame};
use crate::parameter::Parameter;
use crate::playback_state_manager::PlaybackStateManager;
use crate::sound::transport::Transport;
use crate::sound::{PlaybackState, Sound};

pub(super) struct StaticSound {
    sample_rate: u32,
    frames: Arc<[Frame]>,
    playback_state_manager: PlaybackStateManager,
    resampler: Resampler,
    transport: Transport,
    fractional_position: f64,
    volume: Parameter<Decibels>,
    shared: Arc<Shared>,
    /// Whether resampling is needed. Determined on first process call.
    /// None means not yet determined.
    needs_resampling: Option<bool>,
}

impl StaticSound {
    #[must_use]
    pub(crate) fn new(data: StaticSoundData) -> Self {
        let settings = data.settings;
        let transport = Transport::new(data.settings.loops, data.num_frames());
        let starting_frame_index = transport.position;
        let position = starting_frame_index as f64 / data.sample_rate as f64;
        let mut sound = Self {
            sample_rate: data.sample_rate,
            frames: data.frames,
            playback_state_manager: PlaybackStateManager::new(),
            resampler: Resampler::new(starting_frame_index),
            transport,
            fractional_position: 0.0,
            volume: Parameter::new(settings.volume),
            shared: Arc::new(Shared {
                state: AtomicU8::new(PlaybackState::Playing as u8),
                position: AtomicU64::new(position.to_bits()),
            }),
            needs_resampling: None,
        };
        // Fill the resample buffer with 3 samples so playback can start immediately.
        for _ in 0..3 {
            sound.update_position();
        }
        sound
    }

    pub(super) fn shared(&self) -> Arc<Shared> {
        self.shared.clone()
    }

    fn update_shared_playback_state(&mut self) {
        self.shared.set_state(self.playback_state_manager.playback_state());
    }

    /// Updates the current frame index by 1 and pushes a new sample to the
    /// resampler.
    fn update_position(&mut self) {
        self.push_frame_to_resampler();
        self.transport.increment_position(num_frames(&self.frames));
        if !self.transport.playing && self.resampler.empty() {
            self.playback_state_manager.mark_as_stopped();
            self.update_shared_playback_state();
        }
    }

    fn push_frame_to_resampler(&mut self) {
        let frame = self
            .transport
            .playing
            .then(|| frame_at_index(self.transport.position, &self.frames).unwrap_or_default());
        self.resampler.push_frame(frame, self.transport.position);
    }
}

impl Sound for StaticSound {
    fn on_start_processing(&mut self) {
        let last_played_frame_position = self.resampler.current_frame_index();
        self.shared.position.store(
            (last_played_frame_position as f64 / self.sample_rate as f64).to_bits(),
            Ordering::SeqCst,
        );
    }

    fn process(&mut self, out: &mut [Frame], dt: f64) {
        if self.needs_resampling.is_none() {
            let backend_sample_rate = (1.0 / dt).round() as u32;
            self.needs_resampling = Some(self.sample_rate != backend_sample_rate);
        }

        // Update parameters
        self.volume.update(dt * out.len() as f64);
        let changed_playback_state = self.playback_state_manager.update(dt * out.len() as f64);
        if changed_playback_state {
            self.update_shared_playback_state();
        }

        if !self.playback_state_manager.playback_state().is_advancing() {
            out.fill(Frame::ZERO);
            return;
        }

        // Playback audio
        let output_frame_count = out.len();

        match self.needs_resampling {
            Some(false) => {
                // Fast path: no resampling needed.
                for (i, frame) in out.iter_mut().enumerate() {
                    let time_in_chunk = (i + 1) as f64 / output_frame_count as f64;
                    let volume = self.volume.interpolated_value(time_in_chunk).as_amplitude();
                    let fade_volume = self.playback_state_manager.interpolated_fade_volume(time_in_chunk).as_amplitude();

                    let current_frame = if self.transport.playing {
                        frame_at_index(self.transport.position, &self.frames).unwrap_or_default()
                    } else {
                        Frame::ZERO
                    };

                    self.transport.increment_position(num_frames(&self.frames));
                    if !self.transport.playing {
                        self.playback_state_manager.mark_as_stopped();
                        self.update_shared_playback_state();
                    }

                    *frame = current_frame * fade_volume * volume;
                }
            }
            _ => {
                // Resampling path: use interpolation.
                for (i, frame) in out.iter_mut().enumerate() {
                    let time_in_chunk = (i + 1) as f64 / output_frame_count as f64;
                    let volume = self.volume.interpolated_value(time_in_chunk).as_amplitude();
                    let fade_volume = self.playback_state_manager.interpolated_fade_volume(time_in_chunk).as_amplitude();
                    let resampler_out = self.resampler.get(self.fractional_position as f32);
                    self.fractional_position += self.sample_rate as f64 * dt;

                    while self.fractional_position >= 1.0 {
                        self.fractional_position -= 1.0;
                        self.update_position();
                    }

                    *frame = resampler_out * fade_volume * volume;
                }
            }
        }
    }

    fn finished(&self) -> bool {
        self.playback_state_manager.playback_state() == PlaybackState::Stopped
    }
}

pub(super) struct Shared {
    state: AtomicU8,
    position: AtomicU64,
}

impl Shared {
    pub(crate) fn state(&self) -> PlaybackState {
        match self.state.load(Ordering::SeqCst) {
            0 => PlaybackState::Playing,
            1 => PlaybackState::Stopping,
            2 => PlaybackState::Stopped,
            _ => panic!("Invalid playback state"),
        }
    }

    pub(crate) fn set_state(&self, state: PlaybackState) {
        self.state.store(state as u8, Ordering::SeqCst);
    }
}

#[derive(Clone, Copy, PartialEq)]
struct RecentFrame {
    /// A frame of audio.
    frame: Frame,
    /// The current frame index of the source sound at the
    /// time this frame was pushed to the resampler.
    frame_index: usize,
}

pub(super) struct Resampler {
    frames: [RecentFrame; 4],
    time_until_empty: usize,
}

impl Resampler {
    #[must_use]
    pub(crate) fn new(starting_frame_index: usize) -> Self {
        Self {
            frames: [RecentFrame {
                frame: Frame::ZERO,
                frame_index: starting_frame_index,
            }; 4],
            time_until_empty: 0,
        }
    }

    pub(crate) fn push_frame(&mut self, frame: Option<Frame>, sample_index: usize) {
        if frame.is_some() {
            self.time_until_empty = 4;
        } else {
            self.time_until_empty = self.time_until_empty.saturating_sub(1);
        }
        let frame = frame.unwrap_or_default();
        self.frames.copy_within(1.., 0);
        self.frames[self.frames.len() - 1] = RecentFrame {
            frame,
            frame_index: sample_index,
        };
    }

    #[must_use]
    pub(crate) fn get(&self, fractional_position: f32) -> Frame {
        interpolate_frame(
            self.frames[0].frame,
            self.frames[1].frame,
            self.frames[2].frame,
            self.frames[3].frame,
            fractional_position,
        )
    }

    /// Returns the index of the frame in the source sound that the user is
    /// currently hearing from this resampler.
    ///
    /// This is not the same as the most recently pushed frame.
    /// The user mainly hears a frame between `self.frames[1]` and
    /// `self.frames[2]`. `self.frames[0]` and `self.frames[3]` are used to
    /// provide additional information to the interpolation algorithm to get a
    /// smoother result.
    #[must_use]
    pub(crate) fn current_frame_index(&self) -> usize {
        self.frames[1].frame_index
    }

    #[must_use]
    pub(crate) fn empty(&self) -> bool {
        self.time_until_empty == 0
    }
}
