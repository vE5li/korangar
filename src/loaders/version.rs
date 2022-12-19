use std::fmt::{Display, Formatter, Result};

use derive_new::new;

use crate::loaders::ByteConvertable;

#[derive(Copy, Clone, Debug, new)]
pub struct Version {
    pub major: u8,
    pub minor: u8,
}

impl ByteConvertable for Version {
    fn from_bytes(byte_stream: &mut super::ByteStream, length_hint: Option<usize>) -> Self {
        assert!(length_hint.is_none());

        // TODO: this should be the other way around?
        let minor = byte_stream.next();
        let major = byte_stream.next();

        Self::new(major, minor)
    }
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
}

impl Display for Version {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result {
        write!(formatter, "{}.{}", self.major, self.minor)
    }
}
