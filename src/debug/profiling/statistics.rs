use std::collections::HashMap;
use std::ops::Div;
use std::time::Duration;

use super::measurement::Measurement;
use super::{ProfilerThread, DEFERRED_THREAD_PROFILER, MAIN_THREAD_PROFILER, PICKER_THREAD_PROFILER, SHADOW_THREAD_PROFILER};

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

fn process_timing<const RECURSE: bool>(measurement: &Measurement, timings: &mut HashMap<&'static str, MeasurementTiming>) {
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

    if RECURSE {
        measurement
            .indices
            .iter()
            .for_each(|measurement| process_timing::<RECURSE>(measurement, timings));
    }
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

pub fn get_statistics_data(thread: ProfilerThread) -> (Vec<FrameData>, HashMap<&'static str, MeasurementStatistics>, Duration) {
    let profiler = match thread {
        ProfilerThread::Main => unsafe { MAIN_THREAD_PROFILER.lock().unwrap() },
        ProfilerThread::Picker => unsafe { PICKER_THREAD_PROFILER.lock().unwrap() },
        ProfilerThread::Shadow => unsafe { SHADOW_THREAD_PROFILER.lock().unwrap() },
        ProfilerThread::Deferred => unsafe { DEFERRED_THREAD_PROFILER.lock().unwrap() },
    };

    let mut longest_frame_time = Duration::default();

    let frame_data = profiler
        .saved_frames
        .iter()
        .map(|measurement| {
            let total_time = measurement.total_time_taken();
            longest_frame_time = longest_frame_time.max(total_time);

            let frame_times = measurement
                .indices
                .iter()
                .map(|entry| (entry.name, entry.total_time_taken()))
                .collect();

            FrameData { frame_times, total_time }
        })
        .collect();

    let mut timing_map = HashMap::new();

    profiler.saved_frames.iter().for_each(|measurement| {
        process_timing::<false>(measurement, &mut timing_map);
        measurement
            .indices
            .iter()
            .for_each(|measurement| process_timing::<false>(measurement, &mut timing_map))
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
