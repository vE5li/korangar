use std::collections::VecDeque;
use std::time::Instant;

use chrono::prelude::*;
use ragnarok_packets::ClientTick;

pub struct GameTimer {
    global_timer: Instant,
    previous_elapsed: f64,
    accumulate_second: f64,
    frame_counter: usize,
    frames_per_second: usize,
    animation_timer: f32,
    day_timer: f32,
    last_client_tick: Instant,
    first_tick_received: bool,
    base_client_tick: u32,
    frequency: f64,
    last_update: Instant,
    last_error: f64,
    integral_error: f64,
    error_history: VecDeque<(Instant, f64)>,
}

const TIME_FACTOR: f32 = 1000.0;
// PID constants
const KP: f64 = 0.0005;
const KI: f64 = 0.00005;
const KD: f64 = 0.00005;
// Gaussian filter constants
const GAUSSIAN_SIGMA: f64 = 4.0;
const GAUSSIAN_DENOMINATOR: f64 = 2.0 * GAUSSIAN_SIGMA * GAUSSIAN_SIGMA;
const GAUSSIAN_WINDOW_SIZE: usize = 15;

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
            first_tick_received: false,
            base_client_tick: 0,
            frequency: 0.0,
            last_update: Instant::now(),
            last_error: 0.0,
            integral_error: 0.0,
            error_history: VecDeque::with_capacity(GAUSSIAN_WINDOW_SIZE),
        }
    }

    fn gaussian_filter(&self, packet_time: Instant) -> f64 {
        if self.error_history.is_empty() {
            return 0.0;
        }

        let mut weighted_sum = 0.0;
        let mut weight_sum = 0.0;

        for (time, error) in &self.error_history {
            let dt = packet_time.duration_since(*time).as_secs_f64();
            let weight = (-dt.powi(2) / GAUSSIAN_DENOMINATOR).exp();

            weighted_sum += error * weight;
            weight_sum += weight;
        }

        if weight_sum > 0.0 {
            weighted_sum / weight_sum
        } else {
            0.0
        }
    }

    /// Uses a simple PID regulator that uses a gaussian filter to be a bit more
    /// resistant against network jitter to synchronize the client side tick and
    /// the server tick.
    pub fn set_client_tick(&mut self, server_tick: ClientTick, packet_receive_time: Instant) {
        if !self.first_tick_received {
            self.first_tick_received = true;
            self.base_client_tick = server_tick.0;
            self.last_client_tick = packet_receive_time;
            self.last_update = packet_receive_time;
            self.last_error = 0.0;
            self.integral_error = 0.0;
            return;
        }

        let elapsed = packet_receive_time.duration_since(self.last_client_tick).as_secs_f64();
        let adjustment = self.frequency * elapsed;
        let tick_at_receive = self.base_client_tick as f64 + (elapsed * 1000.0) + adjustment;

        let error = server_tick.0 as f64 - tick_at_receive;

        self.error_history.push_back((packet_receive_time, error));
        while self.error_history.len() > GAUSSIAN_WINDOW_SIZE {
            self.error_history.pop_front();
        }

        let filtered_error = self.gaussian_filter(packet_receive_time);

        let dt = packet_receive_time.duration_since(self.last_update).as_secs_f64();

        self.integral_error = (self.integral_error + filtered_error * dt).clamp(-10.0, 10.0);

        let derivative = (filtered_error - self.last_error) / dt;

        self.frequency = (KP * filtered_error + KI * self.integral_error + KD * derivative).clamp(-0.1, 0.1);

        self.last_error = filtered_error;
        self.base_client_tick = server_tick.0;
        self.last_client_tick = packet_receive_time;
        self.last_update = packet_receive_time;
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn get_client_tick(&self) -> ClientTick {
        let elapsed = self.last_client_tick.elapsed().as_secs_f64();
        let adjustment = self.frequency * elapsed;
        let tick = self.base_client_tick as f64 + (elapsed * 1000.0) + adjustment;
        ClientTick(tick as u32)
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
