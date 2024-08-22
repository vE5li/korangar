use std::time::{Duration, Instant};

use super::Profiler;

#[must_use = "ActiveMeasurement must be used, otherwise it will not measure anything"]
pub struct ActiveMeasurement {
    name: &'static str,
}

impl ActiveMeasurement {
    pub fn new(name: &'static str) -> Self {
        Self { name }
    }

    pub fn stop(self) {
        drop(self);
    }
}

impl Drop for ActiveMeasurement {
    fn drop(&mut self) {
        Profiler::stop_measurement(self.name);
    }
}

#[derive(Debug, Clone)]
pub struct Measurement {
    pub name: &'static str,
    pub start_time: Instant,
    pub end_time: Instant,
    pub indices: Vec<usize>,
}

impl Default for Measurement {
    fn default() -> Self {
        let start_time = Instant::now();

        Self {
            name: "",
            start_time,
            end_time: start_time,
            indices: Vec::new(),
        }
    }
}

impl Measurement {
    pub(super) fn start_measurement(&mut self, name: &'static str) {
        let start_time = Instant::now();
        self.name = name;
        self.start_time = start_time;
        self.end_time = start_time;
        self.indices.clear();
    }

    pub(super) fn stop_measurement(&mut self) {
        self.end_time = Instant::now();
    }

    pub fn total_time_taken(&self) -> Duration {
        self.end_time - self.start_time
    }
}
