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
    pub build: u32,
    phantom_data: PhantomData<T>,
}

impl<T> Version<T> {
    pub fn new(major: u8, minor: u8) -> Self {
        Self {
            major,
            minor,
            build: Default::default(),
            phantom_data: PhantomData,
        }
    }
}

impl FromBytes for Version<MajorFirst> {
    fn from_bytes<Meta>(byte_reader: &mut ByteReader<Meta>) -> ConversionResult<Self> {
        let major = byte_reader.byte::<Self>()?;
        let minor = byte_reader.byte::<Self>()?;

        Ok(Version::<MajorFirst>::new(major, minor))
    }
}

impl FromBytes for Version<MinorFirst> {
    fn from_bytes<Meta>(byte_reader: &mut ByteReader<Meta>) -> ConversionResult<Self> {
        let minor = byte_reader.byte::<Self>()?;
        let major = byte_reader.byte::<Self>()?;

        Ok(Version::<MinorFirst>::new(major, minor))
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
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.build)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct InternalVersion {
    pub major: u8,
    pub minor: u8,
    pub build: u32,
}

impl<T> From<Version<T>> for InternalVersion {
    fn from(version: Version<T>) -> Self {
        let Version { major, minor, build, .. } = version;
        Self { major, minor, build }
    }
}

impl InternalVersion {
    pub fn smaller(&self, major: u8, minor: u8) -> bool {
        self.major < major || (self.major == major && self.minor < minor)
    }

    pub fn equals_or_above(&self, major: u8, minor: u8) -> bool {
        self.major > major || (self.major == major && self.minor >= minor)
    }

    pub fn version_build_equals_or_above(&self, major: u8, minor: u8, build: u32) -> bool {
        self.major > major
            || (self.major == major && self.minor > minor)
            || (self.major == major && self.minor == minor && self.build >= build)
    }
}

impl Display for InternalVersion {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.build)
    }
}

#[cfg(test)]
mod conversion {
    use ragnarok_bytes::{ByteReader, ByteWriter, FromBytes, ToBytes};

    use crate::version::{InternalVersion, MajorFirst, MinorFirst, Version};

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

    #[test]
    fn internal_version_smaller() {
        let internal_version: InternalVersion = Version::<MajorFirst>::new(1, 2).into();

        // Equal
        assert!(internal_version.smaller(internal_version.major, internal_version.minor) == false);

        // Minor
        assert!(internal_version.smaller(internal_version.major, internal_version.minor - 1) == false);
        assert!(internal_version.smaller(internal_version.major, internal_version.minor + 1) == true);

        // Major
        assert!(internal_version.smaller(internal_version.major - 1, internal_version.minor) == false);
        assert!(internal_version.smaller(internal_version.major + 1, internal_version.minor) == true);
    }

    #[test]
    fn internal_version_equals_or_above() {
        let internal_version: InternalVersion = Version::<MinorFirst>::new(1, 2).into();

        // Equal
        assert!(internal_version.equals_or_above(internal_version.major, internal_version.minor) == true);

        // Minor
        assert!(internal_version.equals_or_above(internal_version.major, internal_version.minor - 1) == true);
        assert!(internal_version.equals_or_above(internal_version.major, internal_version.minor + 1) == false);

        // Major
        assert!(internal_version.equals_or_above(internal_version.major - 1, internal_version.minor) == true);
        assert!(internal_version.equals_or_above(internal_version.major + 1, internal_version.minor) == false);
    }

    #[test]
    fn internal_version_version_build_equals_or_above() {
        let internal_version_build = InternalVersion {
            major: 2,
            minor: 6,
            build: 187,
        };

        // Equal
        assert!(
            internal_version_build.version_build_equals_or_above(
                internal_version_build.major,
                internal_version_build.minor,
                internal_version_build.build
            ) == true
        );

        // Build
        assert!(
            internal_version_build.version_build_equals_or_above(
                internal_version_build.major,
                internal_version_build.minor,
                internal_version_build.build - 1
            ) == true
        );
        assert!(
            internal_version_build.version_build_equals_or_above(
                internal_version_build.major,
                internal_version_build.minor,
                internal_version_build.build + 1
            ) == false
        );

        // Minor
        assert!(
            internal_version_build.version_build_equals_or_above(
                internal_version_build.major,
                internal_version_build.minor - 1,
                internal_version_build.build
            ) == true
        );
        assert!(
            internal_version_build.version_build_equals_or_above(
                internal_version_build.major,
                internal_version_build.minor + 1,
                internal_version_build.build
            ) == false
        );

        // Major
        assert!(
            internal_version_build.version_build_equals_or_above(
                internal_version_build.major - 1,
                internal_version_build.minor,
                internal_version_build.build
            ) == true
        );
        assert!(
            internal_version_build.version_build_equals_or_above(
                internal_version_build.major + 1,
                internal_version_build.minor,
                internal_version_build.build
            ) == false
        );
    }
}
