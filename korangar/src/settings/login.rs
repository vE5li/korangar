use std::collections::HashMap;

#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, print_debug};
use korangar_interface::element::StateElement;
use ron::ser::PrettyConfig;
use rust_state::{MapItem, RustState};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};

use crate::loaders::ServiceId;

#[derive(Clone, Default, RustState, Serialize, Deserialize, StateElement)]
pub struct LoginSettings {
    // TODO: Unhide this element.
    #[hidden_element]
    pub service_settings: HashMap<ServiceId, ServiceSettings>,
    pub recent_service_id: Option<ServiceId>,
}

impl MapItem for ServiceSettings {
    type Id = ServiceId;
}

#[derive(Clone, Default, RustState, Deserialize)]
pub struct ServiceSettings {
    pub username: String,
    pub password: String,
    pub remember_username: bool,
    pub remember_password: bool,
}

impl Serialize for ServiceSettings {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut serde_state = Serializer::serialize_struct(serializer, "ServiceSettings", 4)?;
        SerializeStruct::serialize_field(
            &mut serde_state,
            "username",
            if self.remember_username { self.username.as_str() } else { "" },
        )?;
        SerializeStruct::serialize_field(
            &mut serde_state,
            "password",
            if self.remember_password { self.password.as_str() } else { "" },
        )?;
        SerializeStruct::serialize_field(&mut serde_state, "remember_username", &self.remember_username)?;
        SerializeStruct::serialize_field(&mut serde_state, "remember_password", &self.remember_password)?;
        SerializeStruct::end(serde_state)
    }
}

impl LoginSettings {
    const FILE_NAME: &'static str = "client/login_settings.ron";

    pub fn new() -> Self {
        Self::load().unwrap_or_else(|| {
            #[cfg(feature = "debug")]
            print_debug!("failed to load login settings from {}", Self::FILE_NAME.magenta());

            Default::default()
        })
    }

    pub fn load() -> Option<Self> {
        #[cfg(feature = "debug")]
        print_debug!("loading login settings from {}", Self::FILE_NAME.magenta());

        std::fs::read_to_string(Self::FILE_NAME)
            .ok()
            .and_then(|data| ron::from_str(&data).ok())
    }

    pub fn save(&self) {
        #[cfg(feature = "debug")]
        print_debug!("saving login settings to {}", Self::FILE_NAME.magenta());

        let data = ron::ser::to_string_pretty(self, PrettyConfig::new()).unwrap();

        if let Err(_error) = std::fs::write(Self::FILE_NAME, data) {
            #[cfg(feature = "debug")]
            print_debug!(
                "failed to save login settings to {}: {:?}",
                Self::FILE_NAME.magenta(),
                _error.red()
            );
        }
    }
}

impl Drop for LoginSettings {
    fn drop(&mut self) {
        self.save();
    }
}
