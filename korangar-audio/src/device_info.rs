use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Condvar, Mutex};

/// Stable identifier for an audio output device, persisted across sessions.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DeviceId(String);

impl DeviceId {
    /// Creates a new device ID from a string.
    pub fn new(id: String) -> Self {
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

/// The user's preferred output device and the list of available devices.
///
/// - Main thread: reads available devices, sets preferred device.
/// - Monitoring thread: updates available devices, reads preferred device.
pub(crate) struct OutputDevicePreference {
    preferred_device: Mutex<Option<DeviceId>>,
    available_devices: Mutex<Vec<DeviceInfo>>,
    devices_changed: AtomicBool,
    wake: (Mutex<bool>, Condvar),
}

impl OutputDevicePreference {
    pub(crate) fn new(preferred: Option<DeviceId>, available: Vec<DeviceInfo>) -> Self {
        Self {
            preferred_device: Mutex::new(preferred),
            available_devices: Mutex::new(available),
            devices_changed: AtomicBool::new(false),
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

    /// Returns the current list of available devices.
    pub fn available_devices(&self) -> Vec<DeviceInfo> {
        self.available_devices.lock().unwrap().clone()
    }

    /// Returns true if the device list has changed since the last call,
    /// clearing the flag.
    pub fn take_devices_changed(&self) -> bool {
        self.devices_changed.swap(false, Ordering::SeqCst)
    }

    /// Updates the available device list. Called by the monitoring thread.
    pub(crate) fn update_available_devices(&self, devices: Vec<DeviceInfo>) {
        let mut current = self.available_devices.lock().unwrap();
        if devices.len() != current.len() || devices.iter().zip(current.iter()).any(|(a, b)| a.id != b.id) {
            *current = devices;
            self.devices_changed.store(true, Ordering::SeqCst);
        }
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
