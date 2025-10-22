use std::time::Duration;

use crate::command::{CommandReader, ValueChangeCommand};
use crate::tween::Tweenable;

/// Manages and updates a value that can be smoothly transitioned.
///
/// You'll only need to use this if you're creating your own
/// [`Sound`](crate::sound::Sound) implementations. If
/// you want to adjust a parameter of something from gameplay code (such as the
/// volume of a sound or the speed of a clock), use the functions on that
/// object's handle.
#[derive(Clone)]
pub(crate) struct Parameter<T: Tweenable = f64> {
    state: State<T>,
    raw_value: T,
    previous_raw_value: T,
}

impl<T: Tweenable> Parameter<T> {
    /// Creates a new [`Parameter`] with an initial value.
    #[must_use]
    pub(crate) fn new(initial_value: T) -> Self {
        Self {
            state: State::Idle { value: initial_value },
            raw_value: initial_value,
            previous_raw_value: initial_value,
        }
    }

    /// Returns the current actual value of the parameter.
    #[must_use]
    pub(crate) fn value(&self) -> T {
        self.raw_value
    }

    /// Returns the previous actual value of the parameter.
    #[must_use]
    pub(crate) fn previous_value(&self) -> T {
        self.previous_raw_value
    }

    /// Returns the interpolated value between the previous and current
    /// actual value of the parameter.
    #[must_use]
    pub(crate) fn interpolated_value(&self, amount: f64) -> T {
        T::interpolate(self.previous_raw_value, self.raw_value, amount)
    }

    /// Starts a transition from the current value to the target value.
    pub(crate) fn set(&mut self, target: T, tween_duration: Duration) {
        self.state = State::Tweening {
            start: self.value(),
            target,
            time: 0.0,
            tween_duration: tween_duration.as_secs_f64(),
        };
    }

    /// Reads a [`ValueChangeCommand`] from a [`CommandReader`], and if there is
    /// one, sets the parameter with the value and tween.
    pub(crate) fn read_command(&mut self, command_reader: &mut CommandReader<ValueChangeCommand<T>>)
    where
        T: Send,
    {
        if let Some(ValueChangeCommand { target, tween_duration }) = command_reader.read() {
            self.set(target, tween_duration);
        }
    }

    /// Updates any in-progress transitions.
    ///
    /// Returns `true` if a transition just finished after this update.
    pub(crate) fn update(&mut self, dt: f64) -> JustFinishedTween {
        self.previous_raw_value = self.raw_value;
        let just_finished_tween = self.update_tween(dt);
        self.raw_value = self.calculate_new_raw_value();
        just_finished_tween
    }

    fn update_tween(&mut self, dt: f64) -> JustFinishedTween {
        if let State::Tweening {
            target,
            time,
            tween_duration,
            ..
        } = &mut self.state
        {
            *time += dt;
            if *time >= *tween_duration {
                self.state = State::Idle { value: *target };
                return true;
            }
        }
        false
    }

    fn calculate_new_raw_value(&self) -> T {
        match &self.state {
            State::Idle { value } => *value,
            State::Tweening {
                start,
                target,
                time,
                tween_duration,
            } => {
                if *tween_duration == 0.0 {
                    *target
                } else {
                    T::interpolate(*start, *target, time / tween_duration)
                }
            }
        }
    }
}

#[derive(Clone)]
enum State<T: Tweenable> {
    Idle {
        value: T,
    },
    Tweening {
        start: T,
        target: T,
        time: f64,
        tween_duration: f64,
    },
}

type JustFinishedTween = bool;
