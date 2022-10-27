use std::time::Instant;

use derive_new::new;

#[derive(new)]
pub struct GameTimer {
    #[new(value = "Instant::now()")]
    global_timer: Instant,
    #[new(default)]
    previous_elapsed: f64,
    #[new(default)]
    accumulate_second: f64,
    #[new(default)]
    frame_counter: usize,
    #[new(default)]
    frames_per_second: usize,
    #[new(default)]
    client_tick: u32,
}

impl GameTimer {
    pub fn set_client_tick(&mut self, client_tick: u32) {
        self.client_tick = client_tick;
    }

    pub fn get_client_tick(&self) -> u32 {
        self.client_tick
    }

    pub fn update(&mut self) -> f64 {
        let new_elapsed = self.global_timer.elapsed().as_secs_f64();
        let delta_time = new_elapsed - self.previous_elapsed;

        self.frame_counter += 1;
        self.accumulate_second += delta_time;
        self.previous_elapsed = new_elapsed;

        if self.accumulate_second > 1.0 {
            self.frames_per_second = self.frame_counter;
            self.accumulate_second -= 1.0;
            self.frame_counter = 0;
        }

        self.client_tick += (delta_time * 1075.0) as u32;

        delta_time
    }

    pub fn last_frames_per_second(&self) -> usize {
        self.frames_per_second
    }
}
