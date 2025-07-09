use std::cell::Cell;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;

use super::{ActiveMeasurement, RingBuffer};
use crate::logging::{Colorize, print_debug};
use crate::profiling::frame_measurement::FrameMeasurement;

#[thread_local]
static PROFILER: Cell<Option<&'static Mutex<Profiler>>> = Cell::new(None);
static PROFILER_HALTED: AtomicBool = AtomicBool::new(false);

#[derive(Default)]
pub struct Profiler {
    active_measurements: Vec<usize>,
    latest_frame: FrameMeasurement,
    saved_frames: RingBuffer<FrameMeasurement, { Profiler::SAVED_FRAME_COUNT }>,
}

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
    pub(crate) fn get_saved_frames(&self) -> &RingBuffer<FrameMeasurement, { Self::SAVED_FRAME_COUNT }> {
        &self.saved_frames
    }

    /// Start a new frame by creating a new root measurement.
    #[doc(hidden)]
    pub fn start_frame(&mut self) -> ActiveMeasurement {
        // Make sure that there are no active measurements.
        if self.active_measurements.len() > 1 {
            let frame_measurement = &self.latest_frame;
            let measurement_names = self
                .active_measurements
                .iter()
                .skip(1)
                .copied()
                .map(|index| frame_measurement[index].name)
                .collect::<Vec<&'static str>>()
                .join(", ");

            print_debug!(
                "[{}] active measurements at the start of the frame; measurement names: {}",
                "warning".yellow(),
                measurement_names.magenta(),
            );
        }

        // Make sure that the profiler is not halted and only save the frame data when
        // there is at least one measurement.
        if !PROFILER_HALTED.load(std::sync::atomic::Ordering::Relaxed) && self.latest_frame.has_measurements() {
            self.saved_frames.push_default_or_recycle();

            // Swap the current and the replaced frame to avoid allocating new vectors on
            // the heap.
            std::mem::swap(self.saved_frames.back_mut().unwrap(), &mut self.latest_frame);
        }

        // Start a new frame and root measurement.
        self.latest_frame.clear();
        let name = Self::ROOT_MEASUREMENT_NAME;
        let index = self.latest_frame.new_measurement(name);

        // Set `active_measurements` to a well-defined state.
        self.active_measurements.clear();
        self.active_measurements.push(index);

        ActiveMeasurement::new(name)
    }

    /// Start a new measurement.
    fn start_measurement_inner(&mut self, name: &'static str) -> ActiveMeasurement {
        // Add a new measurement.
        let index = self.latest_frame.new_measurement(name);

        // Get the most recent active measurement.
        let recent_index = self.active_measurements.last().copied().unwrap();
        let measurement = &mut self.latest_frame[recent_index];

        // Add the new index to the parent measurement.
        measurement.indices.push(index);

        // Set the new index as the new most recent active measurement.
        self.active_measurements.push(index);

        ActiveMeasurement::new(name)
    }

    /// Stop a running measurement.
    fn stop_measurement_inner(&mut self, name: &'static str) {
        // Remove the measurement from the list of active measurements.
        let Some(index) = self.active_measurements.pop() else {
            print_debug!(
                "[{}] tried to stop measurement {} but no measurement is active",
                "warning".yellow(),
                name.magenta(),
            );
            return;
        };

        let measurement = &mut self.latest_frame[index];

        // Set the end time of the measurement.
        measurement.stop_measurement();

        // Assert that the names match to emit a warning when something went wrong.
        if !std::ptr::addr_eq(name, measurement.name) {
            print_debug!(
                "[{}] active measurement mismatch; expected {} but got {}",
                "warning".yellow(),
                measurement.name.magenta(),
                name.magenta(),
            );
        }
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
            #[derive(Debug, Clone, Copy, PartialEq, Eq, rust_state::RustState, korangar_interface::element::StateElement)]
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
