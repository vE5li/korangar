use std::collections::HashMap;
use std::ops::Div;
use std::time::Duration;

use super::measurement::Measurement;
use crate::profiling::LockThreadProfiler;
use crate::profiling::frame_measurement::FrameMeasurement;

#[derive(Default, Debug)]
struct MeasurementTiming {
    collected_times: Vec<Duration>,
    total_time: Duration,
    shortest_time: Duration,
    longest_time: Duration,
    times_called: usize,
}

pub struct MeasurementStatistics {
    pub mean: Duration,
    pub standard_deviation: f64,
}

impl MeasurementTiming {
    pub fn mean(&self) -> Duration {
        self.total_time.div_f32(self.times_called as f32)
    }
}

fn process_timing(measurement: &Measurement, timings: &mut HashMap<&'static str, MeasurementTiming>) {
    let total_time = measurement.total_time_taken();
    let timing = timings.entry(measurement.name).or_insert(MeasurementTiming {
        shortest_time: Duration::MAX,
        ..Default::default()
    });

    timing.collected_times.push(total_time);
    timing.shortest_time = timing.shortest_time.min(total_time);
    timing.longest_time = timing.longest_time.max(total_time);
    timing.total_time += total_time;
    timing.times_called += 1;
}

fn calculate_standard_deviation(mean: Duration, times: &[Duration]) -> f64 {
    let mean = mean.as_secs_f64() * 1000.0;

    times
        .iter()
        .map(|time| {
            let diff = mean - time.as_secs_f64() * 1000.0;
            diff * diff
        })
        .sum::<f64>()
        .div(times.len() as f64)
        .sqrt()
}

#[derive(Debug)]
pub struct FrameData {
    pub frame_times: Vec<(&'static str, Duration)>,
    pub total_time: Duration,
}

pub fn get_statistics_data(thread: impl LockThreadProfiler) -> (Vec<FrameData>, HashMap<&'static str, MeasurementStatistics>, Duration) {
    let profiler = thread.lock_profiler();
    let saved_frames = profiler.get_saved_frames();
    let frame_count = saved_frames.len();
    let mut longest_frame_time = Duration::default();

    let frame_data = saved_frames
        .iter()
        .take(frame_count)
        .map(|frame_measurement| {
            let root_measurement = frame_measurement.root_measurement();
            let total_time = root_measurement.total_time_taken();
            longest_frame_time = longest_frame_time.max(total_time);

            let frame_times = root_measurement
                .indices
                .iter()
                .map(|index| {
                    let measurement = &frame_measurement[*index];
                    (measurement.name, measurement.total_time_taken())
                })
                .collect();

            FrameData { frame_times, total_time }
        })
        .collect();

    let mut timing_map = HashMap::new();

    saved_frames.iter().take(frame_count).for_each(|frame_measurement| {
        let root_measurement = frame_measurement.root_measurement();
        process_timing(root_measurement, &mut timing_map);
        root_measurement.indices.iter().for_each(|index| {
            let measurement = &frame_measurement[*index];
            process_timing(measurement, &mut timing_map)
        });
    });

    let statistics_map = timing_map
        .iter()
        .map(|(name, measurement)| {
            let mean = measurement.mean();
            let standard_deviation = calculate_standard_deviation(mean, &measurement.collected_times);
            (*name, MeasurementStatistics { mean, standard_deviation })
        })
        .collect();

    (frame_data, statistics_map, longest_frame_time)
}

pub fn get_number_of_saved_frames(thread: impl LockThreadProfiler) -> usize {
    let profiler = thread.lock_profiler();
    profiler.get_saved_frames().len()
}

pub fn get_frame_by_index(thread: impl LockThreadProfiler, index: usize) -> FrameMeasurement {
    let profiler = thread.lock_profiler();
    let frames = profiler.get_saved_frames();
    frames[index].clone()
}
