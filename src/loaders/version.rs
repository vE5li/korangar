use std::fmt::{Display, Formatter, Result};
use std::marker::PhantomData;

use derive_new::new;

use super::ByteConvertable;

#[derive(Copy, Clone, Debug)]
pub struct MajorFirst;
#[derive(Copy, Clone, Debug)]
pub struct MinorFirst;
#[derive(Copy, Clone, Debug)]
pub struct Version<T> {
    pub major: u8,
    pub minor: u8,
    phantom_data: PhantomData<T>,
}

impl ByteConvertable for Version<MajorFirst> {
    fn from_bytes(byte_stream: &mut super::ByteStream, length_hint: Option<usize>) -> Self {
        assert!(length_hint.is_none());

        let major = byte_stream.next();
        let minor = byte_stream.next();

        Self {
            major,
            minor,
            phantom_data: PhantomData,
        }
    }
}

impl ByteConvertable for Version<MinorFirst> {
    fn from_bytes(byte_stream: &mut super::ByteStream, length_hint: Option<usize>) -> Self {
        assert!(length_hint.is_none());

        let minor = byte_stream.next();
        let major = byte_stream.next();

        Self {
            minor,
            major,
            phantom_data: PhantomData,
        }
    }
}

impl<T> Display for Version<T> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result {
        write!(formatter, "{}.{}", self.major, self.minor)
    }
}

#[derive(Copy, Clone, Debug, new)]
pub struct InternalVersion {
    pub major: u8,
    pub minor: u8,
}

impl<T> From<Version<T>> for InternalVersion {
    fn from(version: Version<T>) -> Self {
        let Version { major, minor, .. } = version;
        Self { major, minor }
    }
}

impl InternalVersion {
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

impl Display for InternalVersion {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result {
        write!(formatter, "{}.{}", self.major, self.minor)
    }
}
