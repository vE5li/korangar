pub(crate) mod decode_scheduler;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};
use std::time::Duration;

use rtrb::Consumer;

use self::decode_scheduler::DecodeScheduler;
use super::{CommandReaders, StreamingSoundSettings};
use crate::decibels::Decibels;
use crate::frame::{Frame, interpolate_frame};
use crate::parameter::Parameter;
use crate::playback_state_manager::PlaybackStateManager;
use crate::sound::{PlaybackState, Sound};

pub(crate) struct Shared {
    state: AtomicU8,
    position: AtomicU64,
    reached_end: AtomicBool,
    encountered_error: AtomicBool,
}

impl Shared {
    #[must_use]
    pub(crate) fn new() -> Self {
        Self {
            position: AtomicU64::new(0.0f64.to_bits()),
            state: AtomicU8::new(PlaybackState::Playing as u8),
            reached_end: AtomicBool::new(false),
            encountered_error: AtomicBool::new(false),
        }
    }

    #[must_use]
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

    #[must_use]
    pub(crate) fn reached_end(&self) -> bool {
        self.reached_end.load(Ordering::SeqCst)
    }

    #[must_use]
    pub(crate) fn encountered_error(&self) -> bool {
        self.encountered_error.load(Ordering::SeqCst)
    }
}

pub(crate) struct StreamingSound {
    command_readers: CommandReaders,
    sample_rate: u32,
    frame_consumer: Consumer<TimestampedFrame>,
    playback_state_manager: PlaybackStateManager,
    current_frame: usize,
    fractional_position: f64,
    volume: Parameter<Decibels>,
    shared: Arc<Shared>,
    needs_resampling: Option<bool>,
}

impl StreamingSound {
    #[must_use]
    pub(super) fn new(
        sample_rate: u32,
        settings: StreamingSoundSettings,
        shared: Arc<Shared>,
        frame_consumer: Consumer<TimestampedFrame>,
        command_readers: CommandReaders,
        scheduler: &DecodeScheduler,
    ) -> Self {
        let current_frame = scheduler.current_frame();
        let start_position = current_frame as f64 / sample_rate as f64;
        shared.position.store(start_position.to_bits(), Ordering::SeqCst);
        Self {
            command_readers,
            sample_rate,
            frame_consumer,
            playback_state_manager: PlaybackStateManager::new(),
            current_frame,
            fractional_position: 0.0,
            volume: Parameter::new(settings.volume),
            shared,
            needs_resampling: None,
        }
    }

    fn update_shared_playback_state(&mut self) {
        self.shared.set_state(self.playback_state_manager.playback_state());
    }

    fn update_current_frame(&mut self) {
        let chunk = self.frame_consumer.read_chunk(self.frame_consumer.slots().min(4)).unwrap();
        let (a, b) = chunk.as_slices();
        let mut iter = a.iter().chain(b.iter());
        if let Some(TimestampedFrame { index, .. }) = iter.nth(1) {
            self.current_frame = *index;
        }
    }

    #[must_use]
    fn next_frames(&mut self) -> [Frame; 4] {
        let mut frames = [Frame::ZERO; 4];
        let chunk = self.frame_consumer.read_chunk(self.frame_consumer.slots().min(4)).unwrap();
        let (a, b) = chunk.as_slices();
        let mut iter = a.iter().chain(b.iter());
        for frame in &mut frames {
            *frame = iter
                .next()
                .copied()
                .map(|TimestampedFrame { frame, .. }| frame)
                .unwrap_or(Frame::ZERO);
        }
        frames
    }

    #[must_use]
    fn position(&self) -> f64 {
        (self.current_frame as f64 + self.fractional_position) / self.sample_rate as f64
    }

    fn stop(&mut self, fade_out_tween_duration: Duration) {
        self.playback_state_manager.stop(fade_out_tween_duration);
        self.update_shared_playback_state();
    }

    fn read_commands(&mut self) {
        if let Some(tween_duration) = self.command_readers.stop.read() {
            self.stop(tween_duration);
        }
    }
}

impl Sound for StreamingSound {
    fn on_start_processing(&mut self) {
        self.update_current_frame();
        self.shared.position.store(self.position().to_bits(), Ordering::SeqCst);
        self.read_commands();
    }

    fn process(&mut self, out: &mut [Frame], dt: f64) {
        if self.needs_resampling.is_none() {
            let backend_sample_rate = (1.0 / dt).round() as u32;
            self.needs_resampling = Some(self.sample_rate != backend_sample_rate);
        }

        if self.shared.encountered_error() {
            self.playback_state_manager.mark_as_stopped();
            self.update_shared_playback_state();
            out.fill(Frame::ZERO);
            return;
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
        // Pause playback while waiting for audio data. The first frame in the
        // ring-buffer is the previous frame, so we need to make sure there's at least 2
        // before we continue playing.
        if self.frame_consumer.slots() < 2 && !self.shared.reached_end() {
            out.fill(Frame::ZERO);
            return;
        }

        let num_frames = out.len();

        match self.needs_resampling {
            Some(false) => {
                // Fast path: no resampling needed.
                for (i, frame) in out.iter_mut().enumerate() {
                    let time_in_chunk = (i + 1) as f64 / num_frames as f64;
                    let volume = self.volume.interpolated_value(time_in_chunk).as_amplitude();
                    let fade_volume = self.playback_state_manager.interpolated_fade_volume(time_in_chunk).as_amplitude();

                    let current_frame = self
                        .frame_consumer
                        .pop()
                        .map(|timestamped_frame| timestamped_frame.frame)
                        .unwrap_or(Frame::ZERO);

                    if self.shared.reached_end() && self.frame_consumer.is_empty() {
                        self.playback_state_manager.mark_as_stopped();
                        self.update_shared_playback_state();
                    }

                    *frame = current_frame * fade_volume * volume;
                }
            }
            _ => {
                // Resampling path: use interpolation.
                for (i, frame) in out.iter_mut().enumerate() {
                    let time_in_chunk = (i + 1) as f64 / num_frames as f64;
                    let volume = self.volume.interpolated_value(time_in_chunk).as_amplitude();
                    let fade_volume = self.playback_state_manager.interpolated_fade_volume(time_in_chunk).as_amplitude();
                    let next_frames = self.next_frames();
                    let interpolated_out = interpolate_frame(
                        next_frames[0],
                        next_frames[1],
                        next_frames[2],
                        next_frames[3],
                        self.fractional_position as f32,
                    );
                    self.fractional_position += self.sample_rate as f64 * dt;

                    while self.fractional_position >= 1.0 {
                        self.fractional_position -= 1.0;
                        self.frame_consumer.pop().ok();
                    }

                    if self.shared.reached_end() && self.frame_consumer.is_empty() {
                        self.playback_state_manager.mark_as_stopped();
                        self.update_shared_playback_state();
                    }

                    *frame = interpolated_out * fade_volume * volume;
                }
            }
        }
    }

    fn finished(&self) -> bool {
        self.playback_state_manager.playback_state() == PlaybackState::Stopped
    }
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct TimestampedFrame {
    frame: Frame,
    index: usize,
}
