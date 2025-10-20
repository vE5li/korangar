use std::time::Duration;

use crate::decibels::Decibels;
use crate::parameter::Parameter;
use crate::sound::PlaybackState;

pub(crate) struct PlaybackStateManager {
    state: State,
    volume_fade: Parameter<Decibels>,
}

impl PlaybackStateManager {
    pub(crate) fn new() -> Self {
        Self {
            state: State::Playing,
            volume_fade: Parameter::new(Decibels::IDENTITY),
        }
    }

    pub(crate) fn interpolated_fade_volume(&self, amount: f64) -> Decibels {
        self.volume_fade.interpolated_value(amount)
    }

    pub(crate) fn playback_state(&self) -> PlaybackState {
        match self.state {
            State::Playing => PlaybackState::Playing,
            State::Stopping => PlaybackState::Stopping,
            State::Stopped => PlaybackState::Stopped,
        }
    }

    pub(crate) fn stop(&mut self, fade_out_tween_duration: Duration) {
        if let State::Stopped = &self.state {
            return;
        }
        self.state = State::Stopping;
        self.volume_fade.set(Decibels::SILENCE, fade_out_tween_duration);
    }

    pub(crate) fn mark_as_stopped(&mut self) {
        self.state = State::Stopped;
    }

    pub(crate) fn update(&mut self, dt: f64) -> ChangedPlaybackState {
        let finished = self.volume_fade.update(dt);
        match &mut self.state {
            State::Playing => {}
            State::Stopping => {
                if finished {
                    self.state = State::Stopped;
                    return true;
                }
            }
            State::Stopped => {}
        }
        false
    }
}

pub(crate) type ChangedPlaybackState = bool;

enum State {
    Playing,
    Stopping,
    Stopped,
}
