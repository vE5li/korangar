mod measurement;
mod ring_buffer;
mod statistics;

use std::mem::MaybeUninit;
use std::sync::{LazyLock, Mutex, MutexGuard};
use std::time::Instant;

use self::measurement::ActiveMeasurement;
pub use self::measurement::Measurement;
use self::ring_buffer::RingBuffer;
pub use self::statistics::{get_frame_by_index, get_statistics_data, FrameData, MeasurementStatistics};
use crate::debug::*;

#[thread_local]
static mut PROFILER: MaybeUninit<&'static Mutex<Profiler>> = MaybeUninit::uninit();

static mut MAIN_THREAD_PROFILER: LazyLock<Mutex<Profiler>> = LazyLock::new(|| Mutex::new(Profiler::default()));
static mut PICKER_THREAD_PROFILER: LazyLock<Mutex<Profiler>> = LazyLock::new(|| Mutex::new(Profiler::default()));
static mut SHADOW_THREAD_PROFILER: LazyLock<Mutex<Profiler>> = LazyLock::new(|| Mutex::new(Profiler::default()));
static mut DEFERRED_THREAD_PROFILER: LazyLock<Mutex<Profiler>> = LazyLock::new(|| Mutex::new(Profiler::default()));

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfilerThread {
    Main,
    Picker,
    Shadow,
    Deferred,
}

impl ProfilerThread {
    fn lock_profiler(&self) -> MutexGuard<'_, Profiler> {
        match self {
            ProfilerThread::Main => unsafe { MAIN_THREAD_PROFILER.lock().unwrap() },
            ProfilerThread::Picker => unsafe { PICKER_THREAD_PROFILER.lock().unwrap() },
            ProfilerThread::Shadow => unsafe { SHADOW_THREAD_PROFILER.lock().unwrap() },
            ProfilerThread::Deferred => unsafe { DEFERRED_THREAD_PROFILER.lock().unwrap() },
        }
    }
}

pub const ROOT_MEASUREMENT_NAME: &str = "main loop";

#[derive(Default)]
pub struct Profiler {
    root_measurement: Option<Measurement>,
    /// Self referencing pointers
    active_measurements: Vec<*const Measurement>,
    saved_frames: RingBuffer<Measurement, 128>,
}

impl Profiler {
    /// Start a new frame by creating a new root measurement.
    fn start_frame(&mut self) -> ActiveMeasurement {
        // Make sure that there are no active measurements.
        if self.active_measurements.len() > 1 {
            let measurement_names = self
                .active_measurements
                .iter()
                .skip(1)
                .copied()
                .map(|pointer| unsafe { (*pointer).name })
                .collect::<Vec<&'static str>>()
                .join(", ");

            print_debug!(
                "[{}warning{}] active measurements at the start of the frame; measurement names: {}{}{}",
                YELLOW,
                NONE,
                MAGENTA,
                measurement_names,
                NONE,
            );
        }

        // Start a new root measurement.
        let name = ROOT_MEASUREMENT_NAME;
        let previous_measurement = self.root_measurement.replace(Measurement {
            name,
            start_time: Instant::now(),
            end_time: Instant::now(),
            indices: Vec::new(),
        });

        // Set `active_measurements` to a well defined state.
        self.active_measurements = vec![self.root_measurement.as_ref().unwrap() as *const _];

        // Save the completed frame so we can inspect it in the profiler later on.
        if let Some(previous_measurement) = previous_measurement {
            self.saved_frames.push(previous_measurement);
        }

        ActiveMeasurement::new(name)
    }

    /// Start a new measurement.
    fn start_measurement(&mut self, name: &'static str) -> ActiveMeasurement {
        // Get the most recent active measurement.
        let top_measurement = self.active_measurements.last().copied().unwrap();
        let measurement = unsafe { &mut *(top_measurement as *mut Measurement) };

        // Add a new index to the measurement.
        measurement.indices.push(Measurement::new(name));

        // Set the index as the new most recent active measurement.
        let index = measurement.indices.last().unwrap();
        self.active_measurements.push(index as *const _);

        ActiveMeasurement::new(name)
    }

    /// Stop a running measurement.
    fn stop_measurement(&mut self, name: &'static str) {
        // Get the most recent active measurement.
        let top_measurement = self.active_measurements.last().copied().unwrap();
        let measurement = unsafe { &mut *(top_measurement as *mut Measurement) };

        // Assert that the names match to emit a warning when something went wrong.
        if name as *const _ != measurement.name as *const _ {
            print_debug!(
                "[{}warning{}] active measurement mismatch; exepcted {}{}{} but got {}{}{}",
                YELLOW,
                NONE,
                MAGENTA,
                measurement.name,
                NONE,
                MAGENTA,
                name,
                NONE
            );
        }

        // Set the end time of the measurement.
        measurement.set_end_time();

        // Remove the measurement from the list of active measurements.
        self.active_measurements.pop();
    }
}

pub fn profiler_start_main_thread() -> ActiveMeasurement {
    let profiler = unsafe { &MAIN_THREAD_PROFILER };
    let measurement = profiler.lock().unwrap().start_frame();
    unsafe { PROFILER.write(profiler) };
    measurement
}

pub fn profiler_start_picker_thread() -> ActiveMeasurement {
    let profiler = unsafe { &PICKER_THREAD_PROFILER };
    let measurement = profiler.lock().unwrap().start_frame();
    unsafe { PROFILER.write(profiler) };
    measurement
}

pub fn profiler_start_shadow_thread() -> ActiveMeasurement {
    let profiler = unsafe { &SHADOW_THREAD_PROFILER };
    let measurement = profiler.lock().unwrap().start_frame();
    unsafe { PROFILER.write(profiler) };
    measurement
}

pub fn profiler_start_deferred_thread() -> ActiveMeasurement {
    let profiler = unsafe { &DEFERRED_THREAD_PROFILER };
    let measurement = profiler.lock().unwrap().start_frame();
    unsafe { PROFILER.write(profiler) };
    measurement
}

pub fn start_measurement(name: &'static str) -> ActiveMeasurement {
    unsafe { PROFILER.assume_init_ref().lock().unwrap().start_measurement(name) }
}

macro_rules! profile_block {
    ($name:expr) => {
        #[cfg(feature = "debug")]
        let _measurement = crate::debug::start_measurement($name);
    };
}
