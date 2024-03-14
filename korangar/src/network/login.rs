use std::collections::HashMap;

use ron::ser::PrettyConfig;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};

#[cfg(feature = "debug")]
use crate::debug::*;
use crate::loaders::ServiceId;

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct LoginSettings {
    pub service: String,
    pub service_settings: HashMap<ServiceId, ServiceSettings>,
    pub recent_service_id: Option<ServiceId>,
}

#[derive(Clone, Default, Deserialize)]
pub struct ServiceSettings {
    pub username: String,
    pub password: String,
    pub remember_username: bool,
    pub remember_password: bool,
}

impl Serialize for ServiceSettings {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut serde_state = Serializer::serialize_struct(serializer, "ServiceSettings", 4)?;
        SerializeStruct::serialize_field(
            &mut serde_state,
            "username",
            self.remember_username.then_some(self.username.as_str()).unwrap_or_default(),
        )?;
        SerializeStruct::serialize_field(
            &mut serde_state,
            "password",
            self.remember_password.then_some(self.password.as_str()).unwrap_or_default(),
        )?;
        SerializeStruct::serialize_field(&mut serde_state, "remember_username", &self.remember_username)?;
        SerializeStruct::serialize_field(&mut serde_state, "remember_password", &self.remember_password)?;
        SerializeStruct::end(serde_state)
    }
}

impl LoginSettings {
    pub fn new() -> Self {
        Self::load().unwrap_or_else(|| {
            #[cfg(feature = "debug")]
            print_debug!("failed to load login settings from {}filename{}", MAGENTA, NONE);

            Default::default()
        })
    }

    pub fn load() -> Option<Self> {
        #[cfg(feature = "debug")]
        print_debug!("loading login settings from {}filename{}", MAGENTA, NONE);

        std::fs::read_to_string("client/login_settings.ron")
            .ok()
            .and_then(|data| ron::from_str(&data).ok())
    }

    pub fn save(&self) {
        #[cfg(feature = "debug")]
        print_debug!("saving login settings to {}filename{}", MAGENTA, NONE);

        let data = ron::ser::to_string_pretty(self, PrettyConfig::new()).unwrap();
        std::fs::write("client/login_settings.ron", data).expect("unable to write file");
    }
}

impl Drop for LoginSettings {
    fn drop(&mut self) {
        self.save();
    }
}
