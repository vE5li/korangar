//! Plays audio using [cpal](https://crates.io/crates/cpal).

mod error;

use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{Device, StreamConfig};
pub(crate) use error::Error;

use crate::device_info::{DeviceId, DeviceInfo, DeviceName};

#[cfg(target_arch = "wasm32")]
mod wasm;
#[cfg(target_arch = "wasm32")]
pub(crate) use wasm::CpalBackend;

#[cfg(not(target_arch = "wasm32"))]
mod desktop;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use desktop::CpalBackend;

/// A resolved output device paired with its stream configuration.
pub(crate) struct OutputDevice {
    pub device: Device,
    pub config: StreamConfig,
}

impl OutputDevice {
    /// Returns the display name of this device.
    pub fn name(&self) -> DeviceName {
        device_name(&self.device)
    }

    /// Returns the stable ID of this device.
    pub fn id(&self) -> DeviceId {
        device_id(&self.device)
    }

    /// Returns the full device info.
    pub fn device_info(&self) -> DeviceInfo {
        DeviceInfo {
            id: self.id(),
            name: self.name(),
            sample_rate: self.config.sample_rate,
            channels: self.config.channels,
        }
    }

    /// Returns the system default output device.
    pub fn default() -> Result<Self, Error> {
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or(Error::NoDefaultOutputDevice)?;
        let config: StreamConfig = device.default_output_config()?.into();
        Ok(Self { device, config })
    }

    /// Finds a specific output device by its stable ID. Falls back to the
    /// system default if the requested device is not found.
    pub fn by_id(target_id: &DeviceId) -> Result<Self, Error> {
        let host = cpal::default_host();
        if let Ok(devices) = host.output_devices() {
            for device in devices {
                if device_id(&device) == *target_id {
                    let config: StreamConfig = device.default_output_config()?.into();
                    return Ok(Self { device, config });
                }
            }
        }
        Self::default()
    }

    /// Resolves the target device based on a preferred device ID. If a
    /// preferred device is set and available, returns it. Otherwise returns
    /// the system default device.
    pub fn resolve(preferred: Option<&DeviceId>) -> Result<Self, Error> {
        match preferred {
            Some(id) => Self::by_id(id),
            None => Self::default(),
        }
    }

    /// Returns info for all available output devices.
    pub fn list_all() -> Vec<DeviceInfo> {
        let host = cpal::default_host();
        let Ok(devices) = host.output_devices() else {
            return Vec::new();
        };
        devices.filter_map(|d| {
            let config: StreamConfig = d.default_output_config().ok()?.into();
            Some(DeviceInfo {
                id: device_id(&d),
                name: device_name(&d),
                sample_rate: config.sample_rate,
                channels: config.channels,
            })
        }).collect()
    }
}

/// Extracts the display name from a raw cpal device.
fn device_name(device: &Device) -> DeviceName {
    let name = device
        .description()
        .map(|d| d.name().to_string())
        .unwrap_or_else(|_| "Unknown".to_string());
    DeviceName::new(name)
}

/// Extracts the stable ID from a raw cpal device, with a name-based fallback.
fn device_id(device: &Device) -> DeviceId {
    let id_string = device
        .id()
        .map(|id| id.to_string())
        .unwrap_or_else(|_| format!("fallback:{}", device_name(device)));
    DeviceId::new(id_string)
}
