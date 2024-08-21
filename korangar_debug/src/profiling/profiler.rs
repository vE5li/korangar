use std::cell::Cell;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;
use std::time::Instant;

use super::{ActiveMeasurement, Measurement, RingBuffer};
use crate::logging::{print_debug, Colorize};

#[thread_local]
static PROFILER: Cell<Option<&'static Mutex<Profiler>>> = Cell::new(None);
static PROFILER_HALTED: AtomicBool = AtomicBool::new(false);

#[derive(Default)]
pub struct Profiler {
    // Safety: The profiler has self-referencing fields, so we must make sure that the memory of
    // these fields never move: The backing fields inside the inner profiler are saved on the heap
    // and are not accessible by the outer API.
    inner: Box<ProfilerInner>,
}

#[derive(Default)]
struct ProfilerInner {
    root_measurement: Option<Measurement>,
    /// Self referencing pointers
    active_measurements: Vec<*const Measurement>,
    saved_frames: RingBuffer<Measurement, { Profiler::SAVED_FRAME_COUNT }>,
}

// Safety: It's in general safe to send a Profiler between threads, since the
// referencing fields only point to inner data and the inner data is not moved
// by sending it to other threads.
unsafe impl Send for ProfilerInner {}

impl Profiler {
    pub const ROOT_MEASUREMENT_NAME: &'static str = "total";
    pub const SAVED_FRAME_COUNT: usize = 128;

    /// Set the active profiler.
    #[doc(hidden)]
    pub fn set_active(profiler: &'static Mutex<Profiler>) {
        PROFILER.set(Some(profiler));
    }

    /// Start a new measurement.
    ///
    /// # Note
    /// Panics when called before having called `start_frame()` for the current
    /// thread.
    pub fn start_measurement(name: &'static str) -> ActiveMeasurement {
        let mut guard = PROFILER.get().unwrap().lock().unwrap();
        guard.start_measurement_inner(name)
    }

    /// Start a new measurement.
    ///
    /// # Note
    /// Panics when called before having called `start_frame()` for the current
    /// thread.
    pub(crate) fn stop_measurement(name: &'static str) {
        let mut guard = PROFILER.get().unwrap().lock().unwrap();
        guard.stop_measurement_inner(name)
    }

    /// Set the profiler halted state.
    pub fn set_halted(running: bool) {
        PROFILER_HALTED.store(running, std::sync::atomic::Ordering::Relaxed);
    }

    /// Get the profiler halted state.
    pub fn get_halted() -> bool {
        PROFILER_HALTED.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Get the measurements of the previous SAVE_FRAME_COUNT frames.
    pub(crate) fn get_saved_frames(&self) -> &RingBuffer<Measurement, { Self::SAVED_FRAME_COUNT }> {
        &self.inner.saved_frames
    }

    /// Start a new frame by creating a new root measurement.
    #[doc(hidden)]
    pub fn start_frame(&mut self) -> ActiveMeasurement {
        // Make sure that there are no active measurements.
        if self.inner.active_measurements.len() > 1 {
            let measurement_names = self
                .inner
                .active_measurements
                .iter()
                .skip(1)
                .copied()
                .map(|pointer| unsafe { (*pointer).name })
                .collect::<Vec<&'static str>>()
                .join(", ");

            print_debug!(
                "[{}] active measurements at the start of the frame; measurement names: {}",
                "warning".yellow(),
                measurement_names.magenta(),
            );
        }

        // Start a new root measurement.
        let name = Self::ROOT_MEASUREMENT_NAME;
        let previous_measurement = self.inner.root_measurement.replace(Measurement {
            name,
            start_time: Instant::now(),
            end_time: Instant::now(),
            indices: Vec::new(),
        });

        // Set `active_measurements` to a well-defined state.
        self.inner.active_measurements = vec![self.inner.root_measurement.as_ref().unwrap() as *const _];

        // Save the completed frame, so we can inspect it in the profiler later on.
        let profiler_halted = PROFILER_HALTED.load(std::sync::atomic::Ordering::Relaxed);
        if let Some(previous_measurement) = previous_measurement
            && !profiler_halted
        {
            self.inner.saved_frames.push(previous_measurement);
        }

        ActiveMeasurement::new(name)
    }

    /// Start a new measurement.
    fn start_measurement_inner(&mut self, name: &'static str) -> ActiveMeasurement {
        // Get the most recent active measurement.
        let top_measurement = self.inner.active_measurements.last().copied().unwrap();
        let measurement = unsafe { &mut *(top_measurement as *mut Measurement) };

        // Add a new index to the measurement.
        measurement.indices.push(Measurement::new(name));

        // Set the index as the new most recent active measurement.
        let index = measurement.indices.last().unwrap();

        self.inner.active_measurements.push(index as *const _);

        ActiveMeasurement::new(name)
    }

    /// Stop a running measurement.
    fn stop_measurement_inner(&mut self, name: &'static str) {
        let Some(top_measurement) = self.inner.active_measurements.last().copied() else {
            print_debug!(
                "[{}] tried to stop measurement {} but no measurement is active",
                "warning".yellow(),
                name.magenta(),
            );
            return;
        };

        let measurement = unsafe { &mut *(top_measurement as *mut Measurement) };

        // Assert that the names match to emit a warning when something went wrong.
        if !std::ptr::addr_eq(name, measurement.name) {
            print_debug!(
                "[{}] active measurement mismatch; exepcted {} but got {}",
                "warning".yellow(),
                measurement.name.magenta(),
                name.magenta(),
            );
        }

        // Set the end time of the measurement.
        measurement.set_end_time();

        // Remove the measurement from the list of active measurements.
        self.inner.active_measurements.pop();
    }
}

/// Implementation detail of the [`create_profiler_threads`] macro.
pub trait LockThreadProfiler {
    /// Lock the profiler corresponding to the variant.
    fn lock_profiler(&self) -> std::sync::MutexGuard<'_, Profiler>;
}

/// Profile the entire block.
#[macro_export]
macro_rules! profile_block {
    ($name:expr) => {
        #[cfg(feature = "debug")]
        let _measurement = $crate::profiling::Profiler::start_measurement($name);
    };
}

/// Create a module containing all the profiler threads.
#[macro_export]
macro_rules! create_profiler_threads {
    ($name:ident, { $($thread:ident,)* $(,)? }) => {
        pub mod $name {
            use std::sync::MutexGuard;
            use $crate::profiling::Profiler;
            use $crate::profiling::LockThreadProfiler;

            mod locks {
                use std::sync::{LazyLock, Mutex};
                use $crate::profiling::Profiler;

                $(
                    #[allow(non_upper_case_globals)]
                    pub(super) static $thread: LazyLock<Mutex<Profiler>> = LazyLock::new(|| {
                        Mutex::new(Profiler::default())
                    });
                )*
            }

            /// Enum of all the profiler threads.
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub enum Enum {
                $($thread),*
            }

            impl LockThreadProfiler for Enum {
                fn lock_profiler(&self) -> MutexGuard<'_, Profiler> {
                    match self {
                        $(Self::$thread => locks::$thread.lock().unwrap()),*
                    }
                }
            }

            $(
                #[allow(non_snake_case)]
                pub mod $thread {
                    use $crate::profiling::ActiveMeasurement;
                    use $crate::profiling::Profiler;

                    /// Start the frame.
                    pub fn start_frame() -> ActiveMeasurement {
                        let profiler = &super::locks::$thread;
                        let measurement = profiler.lock().unwrap().start_frame();
                        Profiler::set_active(profiler);
                        measurement
                    }
                }
            )*
        }
    };
}
