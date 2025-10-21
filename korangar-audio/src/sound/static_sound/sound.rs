use std::sync::Arc;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};

use super::data::StaticSoundData;
use super::{frame_at_index, num_frames};
use crate::decibels::Decibels;
use crate::frame::Frame;
use crate::parameter::Parameter;
use crate::playback_state_manager::PlaybackStateManager;
use crate::sound::transport::Transport;
use crate::sound::{PlaybackState, Sound};

pub(super) struct StaticSound {
    sample_rate: u32,
    frames: Arc<[Frame]>,
    playback_state_manager: PlaybackStateManager,
    transport: Transport,
    volume: Parameter<Decibels>,
    shared: Arc<Shared>,
}

impl StaticSound {
    #[must_use]
    pub(crate) fn new(data: StaticSoundData) -> Self {
        let settings = data.settings;
        let transport = Transport::new(data.settings.loops, data.num_frames());
        let starting_frame_index = transport.position;
        let position = starting_frame_index as f64 / data.sample_rate as f64;

        Self {
            sample_rate: data.sample_rate,
            frames: data.frames,
            playback_state_manager: PlaybackStateManager::new(),
            transport,
            volume: Parameter::new(settings.volume),
            shared: Arc::new(Shared {
                state: AtomicU8::new(PlaybackState::Playing as u8),
                position: AtomicU64::new(position.to_bits()),
            }),
        }
    }

    pub(super) fn shared(&self) -> Arc<Shared> {
        self.shared.clone()
    }

    fn update_shared_playback_state(&mut self) {
        self.shared.set_state(self.playback_state_manager.playback_state());
    }
}

impl Sound for StaticSound {
    fn on_start_processing(&mut self) {
        // Update playback position
        let position = self.transport.position as f64 / self.sample_rate as f64;
        self.shared.position.store(position.to_bits(), Ordering::SeqCst);
    }

    fn process(&mut self, out: &mut [Frame], dt: f64) {
        self.volume.update(dt * out.len() as f64);
        let changed_playback_state = self.playback_state_manager.update(dt * out.len() as f64);
        if changed_playback_state {
            self.update_shared_playback_state();
        }

        if !self.playback_state_manager.playback_state().is_advancing() {
            out.fill(Frame::ZERO);
            return;
        }

        let output_frame_count = out.len();

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
