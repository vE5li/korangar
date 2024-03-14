mod measurement;
mod ring_buffer;
mod statistics;

use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::{LazyLock, Mutex, MutexGuard};
use std::time::Instant;

use self::measurement::ActiveMeasurement;
pub use self::measurement::Measurement;
pub use self::ring_buffer::RingBuffer;
pub use self::statistics::{get_frame_by_index, get_number_of_saved_frames, get_statistics_data};
use crate::debug::*;

#[thread_local]
static mut PROFILER: MaybeUninit<&'static Mutex<Profiler>> = MaybeUninit::uninit();

static mut MAIN_THREAD_PROFILER: LazyLock<Mutex<Profiler>> = LazyLock::new(|| Mutex::new(Profiler::default()));
static mut PICKER_THREAD_PROFILER: LazyLock<Mutex<Profiler>> = LazyLock::new(|| Mutex::new(Profiler::default()));
static mut SHADOW_THREAD_PROFILER: LazyLock<Mutex<Profiler>> = LazyLock::new(|| Mutex::new(Profiler::default()));
static mut DEFERRED_THREAD_PROFILER: LazyLock<Mutex<Profiler>> = LazyLock::new(|| Mutex::new(Profiler::default()));

static mut PROFILER_HALTED: AtomicBool = AtomicBool::new(false);
static mut PROFILER_HALTED_VERSION: AtomicUsize = AtomicUsize::new(0);

pub fn set_profiler_halted(running: bool) {
    unsafe {
        PROFILER_HALTED.store(running, std::sync::atomic::Ordering::Relaxed);
        PROFILER_HALTED_VERSION.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    };
}

pub fn is_profiler_halted() -> bool {
    unsafe { PROFILER_HALTED.load(std::sync::atomic::Ordering::Relaxed) }
}

pub fn get_profiler_halted_version() -> usize {
    unsafe { PROFILER_HALTED_VERSION.load(std::sync::atomic::Ordering::Relaxed) }
}

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

pub const SAVED_FRAME_COUNT: usize = 128;
pub const ROOT_MEASUREMENT_NAME: &str = "total";

#[derive(Default)]
pub struct Profiler {
    root_measurement: Option<Measurement>,
    /// Self referencing pointers
    active_measurements: Vec<*const Measurement>,
    saved_frames: RingBuffer<Measurement, SAVED_FRAME_COUNT>,
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
        let profiler_halted = unsafe { PROFILER_HALTED.load(std::sync::atomic::Ordering::Relaxed) };
        if let Some(previous_measurement) = previous_measurement
            && !profiler_halted
        {
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
        let Some(top_measurement) = self.active_measurements.last().copied() else {
            print_debug!(
                "[{}warning{}] tried to stop measurement {}{}{} but no measurement is active",
                YELLOW,
                NONE,
                MAGENTA,
                name,
                NONE,
            );
            return;
        };

        let measurement = unsafe { &mut *(top_measurement as *mut Measurement) };

        // Assert that the names match to emit a warning when something went wrong.
        if !std::ptr::addr_eq(name, measurement.name) {
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
