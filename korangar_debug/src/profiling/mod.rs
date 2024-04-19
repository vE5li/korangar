mod measurement;
mod ring_buffer;
mod statistics;

use std::marker::PhantomPinned;
use std::mem::MaybeUninit;
use std::pin::Pin;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;
use std::time::Instant;

pub use self::measurement::{ActiveMeasurement, Measurement};
pub use self::ring_buffer::RingBuffer;
pub use self::statistics::{get_frame_by_index, get_number_of_saved_frames, get_statistics_data};
use crate::logging::{print_debug, Colorize};

// TODO: Ideally we could get rid of the `Box` type since we can also allocate
// this on the stack.
#[thread_local]
static mut PROFILER: MaybeUninit<&'static Mutex<Pin<Box<Profiler>>>> = MaybeUninit::uninit();
static mut PROFILER_HALTED: AtomicBool = AtomicBool::new(false);

#[derive(Default)]
pub struct Profiler {
    root_measurement: Option<Measurement>,
    /// Self referencing pointers
    active_measurements: Vec<*const Measurement>,
    saved_frames: RingBuffer<Measurement, { Self::SAVED_FRAME_COUNT }>,
    /// Since the profiler has some self-referencing fields, we want it to be
    /// !Unpin.
    _pin: PhantomPinned,
}

impl Profiler {
    pub const ROOT_MEASUREMENT_NAME: &'static str = "total";
    pub const SAVED_FRAME_COUNT: usize = 128;

    /// Set the active profiler.
    pub fn set_active(profiler: &'static Mutex<Pin<Box<Profiler>>>) {
        unsafe { PROFILER.write(profiler) };
    }

    /// Start a new measurement.
    pub fn start_measurement(name: &'static str) -> ActiveMeasurement {
        let mut guard = unsafe { PROFILER.assume_init_ref().lock().unwrap() };
        guard.as_mut().start_measurement_inner(name)
    }

    /// Set the profiler halted state.
    pub fn set_halted(running: bool) {
        unsafe { PROFILER_HALTED.store(running, std::sync::atomic::Ordering::Relaxed) };
    }

    /// Get the profiler halted state.
    pub fn get_halted() -> bool {
        unsafe { PROFILER_HALTED.load(std::sync::atomic::Ordering::Relaxed) }
    }

    /// Start a new frame by creating a new root measurement.
    pub fn start_frame(self: Pin<&mut Self>) -> ActiveMeasurement {
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
                "[{}] active measurements at the start of the frame; measurement names: {}",
                "warning".yellow(),
                measurement_names.magenta(),
            );
        }

        // SAFETY: We do not move out of self.
        let self_mut = unsafe { self.get_unchecked_mut() };

        // Start a new root measurement.
        let name = Self::ROOT_MEASUREMENT_NAME;
        let previous_measurement = self_mut.root_measurement.replace(Measurement {
            name,
            start_time: Instant::now(),
            end_time: Instant::now(),
            indices: Vec::new(),
        });

        // Set `active_measurements` to a well defined state.
        self_mut.active_measurements = vec![self_mut.root_measurement.as_ref().unwrap() as *const _];

        // Save the completed frame so we can inspect it in the profiler later on.
        let profiler_halted = unsafe { PROFILER_HALTED.load(std::sync::atomic::Ordering::Relaxed) };
        if let Some(previous_measurement) = previous_measurement
            && !profiler_halted
        {
            self_mut.saved_frames.push(previous_measurement);
        }

        ActiveMeasurement::new(name)
    }

    /// Start a new measurement.
    fn start_measurement_inner(self: Pin<&mut Self>, name: &'static str) -> ActiveMeasurement {
        // Get the most recent active measurement.
        let top_measurement = self.active_measurements.last().copied().unwrap();
        let measurement = unsafe { &mut *(top_measurement as *mut Measurement) };

        // Add a new index to the measurement.
        measurement.indices.push(Measurement::new(name));

        // Set the index as the new most recent active measurement.
        let index = measurement.indices.last().unwrap();

        // SAFETY: We do not move out of self.
        let self_mut = unsafe { self.get_unchecked_mut() };
        self_mut.active_measurements.push(index as *const _);

        ActiveMeasurement::new(name)
    }

    /// Stop a running measurement.
    fn stop_measurement(self: Pin<&mut Self>, name: &'static str) {
        let Some(top_measurement) = self.active_measurements.last().copied() else {
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

        // SAFETY: We do not move out of self.
        let self_mut = unsafe { self.get_unchecked_mut() };

        // Remove the measurement from the list of active measurements.
        self_mut.active_measurements.pop();
    }
}

/// Implementation detail of the [`create_profiler_threads`] macro.
pub trait LockThreadProfier {
    /// Lock the profiler corresponding to the variant.
    fn lock_profiler(&self) -> std::sync::MutexGuard<'_, Pin<Box<Profiler>>>;
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
            use std::boxed::Box;
            use std::pin::Pin;
            use std::sync::MutexGuard;
            use $crate::profiling::Profiler;
            use $crate::profiling::LockThreadProfier;

            mod locks {
                use std::boxed::Box;
                use std::pin::Pin;
                use std::sync::{LazyLock, Mutex};
                use $crate::profiling::Profiler;

                $(
                    #[allow(non_upper_case_globals)]
                    pub(super) static mut $thread: LazyLock<Mutex<Pin<Box<Profiler>>>> = LazyLock::new(|| {
                        Mutex::new(Box::pin(Profiler::default()))
                    });
                )*
            }

            /// Enum of all the profiler threads.
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub enum Enum {
                $($thread),*
            }

            impl LockThreadProfier for Enum {
                fn lock_profiler(&self) -> MutexGuard<'_, Pin<Box<Profiler>>> {
                    match self {
                        $(Self::$thread => unsafe { locks::$thread.lock().unwrap() }),*
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
                        let profiler = unsafe { &super::locks::$thread };
                        let measurement = profiler.lock().unwrap().as_mut().start_frame();
                        Profiler::set_active(profiler);
                        measurement
                    }
                }
            )*
        }
    };
}
