use std::time::Instant;

use chrono::prelude::*;
use ragnarok_packets::ClientTick;

pub struct GameTimer {
    global_timer: Instant,
    previous_elapsed: f64,
    accumulate_second: f64,
    frame_counter: usize,
    frames_per_second: usize,
    animation_timer: f64,
    day_timer: f64,
    last_packet_receive_time: Instant,
    first_tick_received: bool,
    base_client_tick: u32,
    frequency: f64,
    last_error: f64,
    integral_error: f64,
}

const TIME_FACTOR: f64 = 1000.0;
// PID constants
const KP: f64 = 0.000001;
const KI: f64 = 0.0000001;
const KD: f64 = 0.0000001;

impl GameTimer {
    pub fn new() -> Self {
        let local: DateTime<Local> = Local::now();
        let day_timer = (local.hour() as f64 / TIME_FACTOR * 60.0 * 60.0) + (local.minute() as f64 / TIME_FACTOR * 60.0);

        Self {
            global_timer: Instant::now(),
            previous_elapsed: Default::default(),
            accumulate_second: Default::default(),
            frame_counter: Default::default(),
            frames_per_second: Default::default(),
            animation_timer: Default::default(),
            day_timer,
            last_packet_receive_time: Instant::now(),
            first_tick_received: false,
            base_client_tick: 0,
            frequency: 0.0,
            last_error: 0.0,
            integral_error: 0.0,
        }
    }

    /// Uses a simple PID regulator that is very insensitive, since we assume
    /// that the server clock is reasonably accurate and that we need to guard
    /// more against network jitter.
    pub fn set_client_tick(&mut self, server_tick: ClientTick, packet_receive_time: Instant) {
        if !self.first_tick_received {
            self.first_tick_received = true;
            self.base_client_tick = server_tick.0;
            self.last_packet_receive_time = packet_receive_time;
            self.last_error = 0.0;
            self.integral_error = 0.0;
            return;
        }

        let elapsed = packet_receive_time.duration_since(self.last_packet_receive_time).as_secs_f64();
        let adjustment = self.frequency * elapsed;
        let tick_at_receive = self.base_client_tick as f64 + (elapsed * 1000.0) + adjustment;

        let error = server_tick.0 as f64 - tick_at_receive;
        let clamped_error = error.clamp(-50.0, 50.0);
        let dt = packet_receive_time.duration_since(self.last_packet_receive_time).as_secs_f64();
        let derivative = (clamped_error - self.last_error) / dt;

        self.integral_error = (self.integral_error + clamped_error * dt).clamp(-100.0, 100.0);
        self.frequency = (KP * clamped_error + KI * self.integral_error + KD * derivative).clamp(-0.01, 0.01);

        let current_tick = self.get_client_tick_f64();

        self.base_client_tick = current_tick as u32;
        self.last_packet_receive_time = packet_receive_time;
        self.last_error = clamped_error;
    }

    fn get_client_tick_f64(&self) -> f64 {
        let elapsed = self.last_packet_receive_time.elapsed().as_secs_f64();
        let adjustment = self.frequency * elapsed;
        self.base_client_tick as f64 + (elapsed * 1000.0) + adjustment
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn get_client_tick(&self) -> ClientTick {
        let tick = self.get_client_tick_f64();
        ClientTick(tick as u32)
    }

    #[cfg(feature = "debug")]
    pub fn set_day_timer(&mut self, day_timer: f32) {
        self.day_timer = day_timer as f64;
    }

    pub fn get_day_timer(&self) -> f32 {
        self.day_timer as f32
    }

    pub fn get_animation_timer(&self) -> f32 {
        self.animation_timer as f32
    }

    pub fn update(&mut self) -> f64 {
        let new_elapsed = self.global_timer.elapsed().as_secs_f64();
        let delta_time = new_elapsed - self.previous_elapsed;

        self.frame_counter += 1;
        self.accumulate_second += delta_time;
        self.day_timer += delta_time / TIME_FACTOR;
        self.animation_timer += delta_time;
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
mod increment {
    use crate::system::GameTimer;

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

        std::thread::sleep(std::time::Duration::from_millis(10));
        game_timer.update();

        let updated_day_timer = game_timer.get_day_timer();
        let updated_animation_timer = game_timer.get_animation_timer();

        assert!(updated_day_timer > day_timer);
        assert!(updated_animation_timer > animation_timer);
    }
}
