use std::fmt::{Display, Formatter};
use std::marker::PhantomData;

use derive_new::new;

use super::{ConversionError, FromBytes, Named};

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

impl Named for Version<MajorFirst> {
    const NAME: &'static str = "Version<MajorFirst>";
}

impl Named for Version<MinorFirst> {
    const NAME: &'static str = "Version<MinorFirst>";
}

impl FromBytes for Version<MajorFirst> {
    fn from_bytes(byte_stream: &mut super::ByteStream) -> Result<Self, Box<ConversionError>> {
        let major = byte_stream.next::<Self>()?;
        let minor = byte_stream.next::<Self>()?;

        Ok(Self {
            major,
            minor,
            phantom_data: PhantomData,
        })
    }
}

impl FromBytes for Version<MinorFirst> {
    fn from_bytes(byte_stream: &mut super::ByteStream) -> Result<Self, Box<ConversionError>> {
        let minor = byte_stream.next::<Self>()?;
        let major = byte_stream.next::<Self>()?;

        Ok(Self {
            minor,
            major,
            phantom_data: PhantomData,
        })
    }
}

impl<T> Display for Version<T> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
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

    pub fn equals_or_above(&self, major: u8, minor: u8) -> bool {
        self.major > major || (self.major == major && self.minor >= minor)
    }
}

impl Display for InternalVersion {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}.{}", self.major, self.minor)
    }
}
