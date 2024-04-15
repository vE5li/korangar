use std::time::{Duration, Instant};

use super::PROFILER;

#[must_use = "ActiveMeasurement must be used, otherwise it will not measure anything"]
pub struct ActiveMeasurement {
    name: &'static str,
}

impl ActiveMeasurement {
    pub fn new(name: &'static str) -> Self {
        Self { name }
    }

    pub fn stop(self) {
        std::mem::drop(self);
    }
}

impl Drop for ActiveMeasurement {
    fn drop(&mut self) {
        unsafe { PROFILER.assume_init_ref().lock().unwrap().stop_measurement(self.name) };
    }
}

#[derive(Debug, Clone)]
pub struct Measurement {
    pub name: &'static str,
    pub start_time: Instant,
    pub end_time: Instant,
    pub indices: Vec<Measurement>,
}

impl Measurement {
    pub fn new(name: &'static str) -> Self {
        let start_time = Instant::now();

        Self {
            name,
            start_time,
            end_time: start_time,
            indices: Vec::new(),
        }
    }

    pub(super) fn set_end_time(&mut self) {
        self.end_time = Instant::now();
    }

    pub fn total_time_taken(&self) -> Duration {
        self.end_time - self.start_time
    }
}
