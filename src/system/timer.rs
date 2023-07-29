use std::time::Instant;

use chrono::prelude::*;
use procedural::profile;

use crate::network::ClientTick;

pub struct GameTimer {
    global_timer: Instant,
    previous_elapsed: f64,
    accumulate_second: f64,
    frame_counter: usize,
    frames_per_second: usize,
    animation_timer: f32,
    day_timer: f32,
    last_client_tick: Instant,
    base_client_tick: u32,
}

const TIME_FACTOR: f32 = 1000.0;

impl GameTimer {
    pub fn new() -> Self {
        let local: DateTime<Local> = Local::now();
        let day_timer = (local.hour() as f32 / TIME_FACTOR * 60.0 * 60.0) + (local.minute() as f32 / TIME_FACTOR * 60.0);

        Self {
            global_timer: Instant::now(),
            previous_elapsed: Default::default(),
            accumulate_second: Default::default(),
            frame_counter: Default::default(),
            frames_per_second: Default::default(),
            animation_timer: Default::default(),
            day_timer,
            last_client_tick: Instant::now(),
            base_client_tick: 0,
        }
    }

    pub fn set_client_tick(&mut self, client_tick: ClientTick) {
        self.last_client_tick = Instant::now();
        self.base_client_tick = client_tick.0;
    }

    #[profile]
    pub fn get_client_tick(&self) -> ClientTick {
        ClientTick(self.last_client_tick.elapsed().as_millis() as u32 + self.base_client_tick)
    }

    #[cfg(feature = "debug")]
    pub fn set_day_timer(&mut self, day_timer: f32) {
        self.day_timer = day_timer;
    }

    pub fn get_day_timer(&self) -> f32 {
        self.day_timer
    }

    pub fn get_animation_timer(&self) -> f32 {
        self.animation_timer
    }

    pub fn update(&mut self) -> f64 {
        let new_elapsed = self.global_timer.elapsed().as_secs_f64();
        let delta_time = new_elapsed - self.previous_elapsed;

        self.frame_counter += 1;
        self.accumulate_second += delta_time;
        self.day_timer += delta_time as f32 / TIME_FACTOR;
        self.animation_timer += delta_time as f32;
        self.previous_elapsed = new_elapsed;

        if self.accumulate_second > 1.0 {
            self.frames_per_second = self.frame_counter;
            self.accumulate_second -= 1.0;
            self.frame_counter = 0;
        }

        delta_time
    }

    #[cfg(feature = "debug")]
    pub fn last_frames_per_second(&self) -> usize {
        self.frames_per_second
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn update_increments_frame_counter() {
        let mut game_timer = GameTimer::new();
        game_timer.update();
        assert_eq!(game_timer.frame_counter, 1);
    }

    #[test]
    fn update_increments_timers() {
        let mut game_timer = GameTimer::new();

        let day_timer = game_timer.get_day_timer();
        let animation_timer = game_timer.get_animation_timer();

        game_timer.update();

        let updated_day_timer = game_timer.get_day_timer();
        let updated_animation_timer = game_timer.get_animation_timer();

        assert!(updated_day_timer > day_timer);
        assert!(updated_animation_timer > animation_timer);
    }
}
