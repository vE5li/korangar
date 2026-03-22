#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, print_debug};
use korangar_interface::components::drop_down::DropDownItem;
use korangar_interface::element::StateElement;
use ron::ser::PrettyConfig;
use rust_state::RustState;
use serde::{Deserialize, Serialize};

/// Identity type for the device dropdown. Index 0 = "System Default",
/// index N = the Nth device in the available devices list.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, RustState, StateElement)]
pub struct OutputDeviceOptionId(pub usize);

/// An entry in the output device dropdown.
#[derive(Debug, Clone, RustState, StateElement)]
pub struct OutputDeviceOption {
    /// Display name shown in the dropdown.
    pub display_name: String,
    /// Device ID string, or None for "System Default".
    pub device_id: Option<String>,
    /// Index in the dropdown list.
    pub index: OutputDeviceOptionId,
}

impl DropDownItem<OutputDeviceOptionId> for OutputDeviceOption {
    fn text(&self) -> &str {
        &self.display_name
    }

    fn value(&self) -> OutputDeviceOptionId {
        self.index
    }
}

#[derive(Clone, Serialize, Deserialize, RustState, StateElement)]
pub struct AudioSettings {
    pub mute_on_focus_loss: bool,
    /// Stable ID of the preferred output device, or None to follow the system default.
    #[serde(default)]
    pub preferred_device_id: Option<String>,
    /// Available output devices for the dropdown. Not persisted.
    #[serde(skip)]
    pub available_output_devices: Vec<OutputDeviceOption>,
    /// Currently selected output device index. Not persisted.
    #[serde(skip)]
    pub selected_output_device: OutputDeviceOptionId,
}

impl AudioSettings {
    /// Sets the available device list and resolves the selected device
    /// from the saved preference.
    pub fn set_device_list(&mut self, device_list: Vec<OutputDeviceOption>) {
        self.selected_output_device = self.preferred_device_id.as_ref()
            .and_then(|saved_id| {
                device_list.iter()
                    .find(|d| d.device_id.as_deref() == Some(saved_id.as_str()))
                    .map(|d| d.index)
            })
            .unwrap_or(OutputDeviceOptionId(0));
        self.available_output_devices = device_list;
    }

    /// Returns the preferred device ID derived from the current selection.
    pub fn selected_device_id(&self) -> Option<&String> {
        self.available_output_devices
            .get(self.selected_output_device.0)
            .and_then(|d| d.device_id.as_ref())
    }
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            mute_on_focus_loss: true,
            preferred_device_id: None,
            available_output_devices: Vec::new(),
            selected_output_device: OutputDeviceOptionId(0),
        }
    }
}

impl AudioSettings {
    const FILE_NAME: &'static str = "client/audio_settings.ron";

    pub fn new() -> Self {
        Self::load().unwrap_or_else(|| {
            #[cfg(feature = "debug")]
            print_debug!("failed to load audio settings from {}", Self::FILE_NAME.magenta());
            Default::default()
        })
    }

    pub fn load() -> Option<Self> {
        #[cfg(feature = "debug")]
        print_debug!("loading audio settings from {}", Self::FILE_NAME.magenta());
        std::fs::read_to_string(Self::FILE_NAME)
            .ok()
            .and_then(|data| ron::from_str(&data).ok())
    }

    pub fn save(&self) {
        #[cfg(feature = "debug")]
        print_debug!("saving audio settings to {}", Self::FILE_NAME.magenta());

        let data = ron::ser::to_string_pretty(self, PrettyConfig::new()).unwrap();

        if let Err(_error) = std::fs::write(Self::FILE_NAME, data) {
            #[cfg(feature = "debug")]
            print_debug!(
                "failed to save audio settings to {}: {:?}",
                Self::FILE_NAME.magenta(),
                _error.red()
            );
        }
    }
}

impl Drop for AudioSettings {
    fn drop(&mut self) {
        self.save();
    }
}
