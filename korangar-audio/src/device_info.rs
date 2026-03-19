use std::fmt;
use std::sync::{Condvar, Mutex};

/// Stable identifier for an audio output device, persisted across sessions.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DeviceId(String);

impl DeviceId {
    pub(crate) fn new(id: String) -> Self {
        Self(id)
    }

    /// Returns the ID as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Human-readable display name for an audio output device.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DeviceName(String);

impl DeviceName {
    pub(crate) fn new(name: String) -> Self {
        Self(name)
    }

    /// Returns the name as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DeviceName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Full description of an output device.
#[derive(Clone, Debug)]
pub struct DeviceInfo {
    /// Stable identifier.
    pub id: DeviceId,
    /// Human-readable name.
    pub name: DeviceName,
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Number of output channels.
    pub channels: u16,
}

/// The user's preferred output device.
///
/// Set by the main thread, read by the monitoring thread.
/// The condvar wakes the monitoring thread immediately on change.
pub(crate) struct OutputDevicePreference {
    preferred_device: Mutex<Option<DeviceId>>,
    wake: (Mutex<bool>, Condvar),
}

impl OutputDevicePreference {
    pub(crate) fn new(preferred: Option<DeviceId>) -> Self {
        Self {
            preferred_device: Mutex::new(preferred),
            wake: (Mutex::new(false), Condvar::new()),
        }
    }

    pub(crate) fn get(&self) -> Option<DeviceId> {
        self.preferred_device.lock().unwrap().clone()
    }

    pub(crate) fn set(&self, device: Option<DeviceId>) {
        *self.preferred_device.lock().unwrap() = device;
        *self.wake.0.lock().unwrap() = true;
        self.wake.1.notify_one();
    }

    /// Waits up to `timeout` for a preference change, or returns immediately
    /// if one was already signaled.
    pub(crate) fn wait_for_change(&self, timeout: std::time::Duration) {
        let guard = self.wake.0.lock().unwrap();
        if !*guard {
            let _ = self.wake.1.wait_timeout(guard, timeout);
        }
        *self.wake.0.lock().unwrap() = false;
    }
}
