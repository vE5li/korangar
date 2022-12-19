use std::fmt::{Display, Formatter, Result};

use derive_new::new;
use procedural::ByteConvertable;

#[derive(Copy, Clone, Debug, ByteConvertable, new)]
pub struct Version {
    pub major: u8,
    pub minor: u8,
}

impl Version {
    pub fn smaller(&self, major: u8, minor: u8) -> bool {
        self.major < major || (self.major == major && self.minor < minor)
    }

    pub fn equals(&self, major: u8, minor: u8) -> bool {
        self.major == major && self.minor >= minor
    }

    pub fn equals_or_above(&self, major: u8, minor: u8) -> bool {
        self.major > major || (self.major == major && self.minor >= minor)
    }

    pub fn get_minor_first(&self) -> Version {
        Self {
            major: self.minor,
            minor: self.major,
        }
    }
}

impl Display for Version {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result {
        write!(formatter, "{}.{}", self.major, self.minor)
    }
}
