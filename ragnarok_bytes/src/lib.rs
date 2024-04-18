#![cfg_attr(test, feature(assert_matches))]
#![feature(const_trait_impl)]

mod error;
mod fixed;
mod from_bytes;
mod stream;
mod to_bytes;

#[cfg(feature = "derive")]
pub use ragnarok_procedural::{ByteConvertable, FixedByteSize, FromBytes, ToBytes};

pub use self::error::{ConversionError, ConversionErrorType, ConversionResult, ConversionResultExt};
pub use self::fixed::{FixedByteSize, FixedByteSizeCollection};
pub use self::from_bytes::{FromBytes, FromBytesExt};
pub use self::stream::ByteStream;
pub use self::to_bytes::{ToBytes, ToBytesExt};
