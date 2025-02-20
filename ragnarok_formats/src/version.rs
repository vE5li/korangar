use std::fmt::{Display, Formatter};
use std::marker::PhantomData;

use ragnarok_bytes::{ByteReader, ByteWriter, ConversionResult, FromBytes, ToBytes};

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

impl<T> Version<T> {
    pub fn new(major: u8, minor: u8) -> Self {
        Self {
            major,
            minor,
            phantom_data: PhantomData,
        }
    }
}

impl FromBytes for Version<MajorFirst> {
    fn from_bytes<Meta>(byte_reader: &mut ByteReader<Meta>) -> ConversionResult<Self> {
        let major = byte_reader.byte::<Self>()?;
        let minor = byte_reader.byte::<Self>()?;

        Ok(Self {
            major,
            minor,
            phantom_data: PhantomData,
        })
    }
}

impl FromBytes for Version<MinorFirst> {
    fn from_bytes<Meta>(byte_reader: &mut ByteReader<Meta>) -> ConversionResult<Self> {
        let minor = byte_reader.byte::<Self>()?;
        let major = byte_reader.byte::<Self>()?;

        Ok(Self {
            minor,
            major,
            phantom_data: PhantomData,
        })
    }
}

impl ToBytes for Version<MajorFirst> {
    fn to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        byte_writer.write_counted(|write| {
            write.push(self.major);
            write.push(self.minor);

            Ok(())
        })
    }
}

impl ToBytes for Version<MinorFirst> {
    fn to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        byte_writer.write_counted(|write| {
            write.push(self.minor);
            write.push(self.major);

            Ok(())
        })
    }
}

impl<T> Display for Version<T> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}.{}", self.major, self.minor)
    }
}

#[derive(Copy, Clone, Debug)]
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

#[cfg(test)]
mod conversion {
    use ragnarok_bytes::{ByteReader, ByteWriter, FromBytes, ToBytes};

    use super::{MajorFirst, Version};
    use crate::version::MinorFirst;

    #[test]
    fn version_major_first() {
        let input = &[4, 7];
        let mut byte_reader = ByteReader::without_metadata(input);

        let version = Version::<MajorFirst>::from_bytes(&mut byte_reader).unwrap();

        let mut byte_writer = ByteWriter::new();
        version.to_bytes(&mut byte_writer).unwrap();

        assert_eq!(input, byte_writer.into_inner().as_slice());
    }

    #[test]
    fn version_minor_first() {
        let input = &[7, 4];
        let mut byte_reader = ByteReader::without_metadata(input);

        let version = Version::<MinorFirst>::from_bytes(&mut byte_reader).unwrap();

        let mut byte_writer = ByteWriter::new();
        version.to_bytes(&mut byte_writer).unwrap();

        assert_eq!(input, byte_writer.into_inner().as_slice());
    }
}
