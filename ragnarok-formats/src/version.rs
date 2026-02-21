use std::fmt::{Display, Formatter};
use std::marker::PhantomData;

use ragnarok_bytes::{
    ByteConvertable, ByteReader, ByteWriter, CastableMetadata, Caster, ConversionResult, DynMetadata, FromBytes, ToBytes,
};

/// Marker trait for [`Version`].
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct MajorFirst;

/// Marker trait for [`Version`].
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct MinorFirst;

/// File version, either [`MajorFirst`] or [`MinorFirst`].
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Version<T> {
    /// Major version.
    pub major: u8,
    /// Minor version.
    pub minor: u8,
    _marker: PhantomData<T>,
}

impl<T> Version<T> {
    /// Creates a new version from the major and minor components.
    pub fn new(major: u8, minor: u8) -> Self {
        Self {
            major,
            minor,
            _marker: PhantomData,
        }
    }
}

impl FromBytes for Version<MajorFirst> {
    fn from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self> {
        let major = byte_reader.byte::<Self>()?;
        let minor = byte_reader.byte::<Self>()?;

        Ok(Self {
            major,
            minor,
            _marker: PhantomData,
        })
    }
}

impl FromBytes for Version<MinorFirst> {
    fn from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self> {
        let minor = byte_reader.byte::<Self>()?;
        let major = byte_reader.byte::<Self>()?;

        Ok(Self {
            minor,
            major,
            _marker: PhantomData,
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

/// Version of [`Version`] without the generic for internal use.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct InternalVersion {
    /// Major version.
    pub major: u8,
    /// Minor version.
    pub minor: u8,
}

impl<T> From<Version<T>> for InternalVersion {
    fn from(version: Version<T>) -> Self {
        let Version { major, minor, .. } = version;
        Self { major, minor }
    }
}

impl InternalVersion {
    /// Returns true if the version is below the version specified by the
    /// function parameters.
    pub fn below(&self, major: u8, minor: u8) -> bool {
        self.major < major || (self.major == major && self.minor < minor)
    }

    /// Returns true if the version is equals or above the version specified by
    /// the function parameters.
    pub fn equals_or_above(&self, major: u8, minor: u8) -> bool {
        self.major > major || (self.major == major && self.minor >= minor)
    }

    /// Returns true if the version is above the version specified by the
    /// function parameters *or* if the version is equal and the extra condition
    /// is true.
    ///
    /// This allows extending the version with additional subversions, for
    /// example the build version of the map format.
    pub fn equals_or_above_with_extra_condition(&self, major: u8, minor: u8, extra_condition: bool) -> bool {
        self.major > major
            || (self.major == major && self.minor >= minor)
            || (self.major == major && self.minor == minor && extra_condition)
    }
}

impl Display for InternalVersion {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}.{}", self.major, self.minor)
    }
}

/// Build version.
///
/// This was added to the map data at some point to conditionally enable some
/// fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct BuildVersion(pub u32);

impl BuildVersion {
    /// Returns true if the version is equals or above the version specified by
    /// the function parameters.
    pub fn equals_or_above(&self, build: u32) -> bool {
        build >= self.0
    }
}

/// Version metadata trait.
///
/// This trait exists to allow types to check the file version during conversion
/// without needing to know the exact metadata of the byte reader. As long as
/// the metadata implements `VersionMetadata` and registers the caster properly
/// in [`CastableMetadata::register`], the version can be written and read.
///
/// See [`GenericFormatMetadata`] and [`MapFormatMetadata`] for an
/// implementation.
pub trait VersionMetadata {
    /// Returns the current version if it was already set, none otherwise.
    fn get_version(&self) -> Option<InternalVersion>;

    /// Sets the current version.
    fn set_version(&mut self, version: InternalVersion);
}

/// Build version metadata trait.
///
/// This trait exists to allow types to check the build version during
/// conversion without needing to know the exact metadata of the byte reader. As
/// long as the metadata implements `BuildVersionMetadata` and registers the
/// caster properly in [`CastableMetadata::register`], the build version can be
/// written and read.
///
/// See [`MapFormatMetadata`] for an implementation.
pub trait BuildVersionMetadata {
    /// Returns the current build version if it was already set, none otherwise.
    fn get_build_version(&self) -> Option<BuildVersion>;

    /// Sets the current build version.
    fn set_build_version(&mut self, build_version: BuildVersion);
}

/// Generic metadata for most file formats.
///
/// Only supports casting to `dyn VersionMetadata`.
#[derive(Debug, Default)]
pub struct GenericFormatMetadata {
    version: Option<InternalVersion>,
}

impl VersionMetadata for GenericFormatMetadata {
    fn get_version(&self) -> Option<InternalVersion> {
        self.version
    }

    fn set_version(&mut self, version: InternalVersion) {
        self.version = Some(version);
    }
}

impl CastableMetadata for GenericFormatMetadata {
    fn register(metadata: &mut DynMetadata) {
        metadata.register_caster(Caster::new(
            |any| any.downcast_ref::<Self>().map(|this| this as &dyn VersionMetadata),
            |any| any.downcast_mut::<Self>().map(|this| this as &mut dyn VersionMetadata),
        ));
    }
}

/// Generic metadata for most file formats.
///
/// Supports casting to `dyn VersionMetadata` and `dyn BuildVersionMetadata`.
#[derive(Debug, Default)]
pub struct MapFormatMetadata {
    version: Option<InternalVersion>,
    build_version: Option<BuildVersion>,
}

impl VersionMetadata for MapFormatMetadata {
    fn get_version(&self) -> Option<InternalVersion> {
        self.version
    }

    fn set_version(&mut self, version: InternalVersion) {
        self.version = Some(version);
    }
}

impl BuildVersionMetadata for MapFormatMetadata {
    fn get_build_version(&self) -> Option<BuildVersion> {
        self.build_version
    }

    fn set_build_version(&mut self, build_version: BuildVersion) {
        self.build_version = Some(build_version);
    }
}

impl CastableMetadata for MapFormatMetadata {
    fn register(metadata: &mut DynMetadata) {
        metadata.register_caster(Caster::new(
            |any| any.downcast_ref::<Self>().map(|this| this as &dyn VersionMetadata),
            |any| any.downcast_mut::<Self>().map(|this| this as &mut dyn VersionMetadata),
        ));
        metadata.register_caster(Caster::new(
            |any| any.downcast_ref::<Self>().map(|this| this as &dyn BuildVersionMetadata),
            |any| any.downcast_mut::<Self>().map(|this| this as &mut dyn BuildVersionMetadata),
        ));
    }
}

#[cfg(test)]
mod conversion {
    use ragnarok_bytes::{ByteReader, ByteWriter, FromBytes, ToBytes};

    use super::{BuildVersion, MajorFirst, MinorFirst, Version};

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
    fn build_version() {
        let input = &[186, 0, 0, 0];
        let mut byte_reader = ByteReader::without_metadata(input);

        let build_version = BuildVersion::from_bytes(&mut byte_reader).unwrap();

        let mut byte_writer = ByteWriter::new();
        build_version.to_bytes(&mut byte_writer).unwrap();

        assert_eq!(input, byte_writer.into_inner().as_slice());
    }
}

#[cfg(test)]
mod comparison {
    use super::{BuildVersion, InternalVersion};

    #[test]
    fn internal_version_below() {
        let version = InternalVersion { major: 2, minor: 5 };

        // Target major is greater.
        assert!(version.below(3, 0));
        assert!(version.below(3, 5));

        // Major is the same, target minor is greater.
        assert!(version.below(2, 6));
        assert!(version.below(2, 7));

        // Version is equal.
        assert!(!version.below(2, 5));

        // Version is greater.
        assert!(!version.below(2, 4));
        assert!(!version.below(1, 9));
    }

    #[test]
    fn internal_version_equals_or_above() {
        let version = InternalVersion { major: 2, minor: 5 };

        // Major is greater.
        assert!(version.equals_or_above(1, 5));
        assert!(version.equals_or_above(1, 9));

        // Major is same, minor is greater or equal.
        assert!(version.equals_or_above(2, 5));
        assert!(version.equals_or_above(2, 4));
        assert!(version.equals_or_above(2, 0));

        // Version is below.
        assert!(!version.equals_or_above(2, 6));
        assert!(!version.equals_or_above(3, 0));
    }

    #[test]
    fn internal_version_equals_or_above_with_extra_condition() {
        let version = InternalVersion { major: 2, minor: 5 };

        // Major version is greater.
        assert!(version.equals_or_above_with_extra_condition(1, 5, false));

        // Major and minor are equal or above.
        assert!(version.equals_or_above_with_extra_condition(2, 5, false));
        assert!(version.equals_or_above_with_extra_condition(2, 4, false));

        // Major and minor match exactly, extra condition true.
        assert!(version.equals_or_above_with_extra_condition(2, 5, true));

        // Major and minor match exactly, extra condition false.
        assert!(version.equals_or_above_with_extra_condition(2, 5, false));

        // Version is below and extra condition is false.
        assert!(!version.equals_or_above_with_extra_condition(2, 6, false));
        assert!(!version.equals_or_above_with_extra_condition(3, 0, false));

        // Version is below but extra condition is true.
        assert!(!version.equals_or_above_with_extra_condition(2, 6, true));
    }

    #[test]
    fn build_version_equals_or_above() {
        let build_version = BuildVersion(186);

        // Version is equal.
        assert!(build_version.equals_or_above(186));

        // Version is above.
        assert!(build_version.equals_or_above(200));

        // Version is below.
        assert!(!build_version.equals_or_above(185));
        assert!(!build_version.equals_or_above(100));
    }
}
