use std::time::Instant;

pub struct FrameTimer {
    global_timer: Instant,
    previous_elapsed: f64,
    accumulate_second: f64,
    frame_counter: usize,
    frames_per_second: usize
}

impl FrameTimer {

    pub fn new() -> Self {

        let global_timer = Instant::now();
        let previous_elapsed = 0.0;
        let accumulate_second = 0.0;
        let frame_counter = 0;
        let frames_per_second = 0;

        return Self { global_timer, previous_elapsed, accumulate_second, frame_counter, frames_per_second };
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

        return delta_time;
    }

    pub fn last_frames_per_second(&self) -> usize {
        return self.frames_per_second;
    }
}
