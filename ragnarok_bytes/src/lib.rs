#![feature(array_try_from_fn)]
#![cfg_attr(test, feature(assert_matches))]

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

#[cfg(test)]
mod conversion {
    use crate::{ByteStream, FromBytes, ToBytes};

    fn encode_decode<T: FromBytes + ToBytes>(input: &[u8]) {
        let mut byte_stream = ByteStream::<()>::without_metadata(input);

        let data = T::from_bytes(&mut byte_stream).unwrap();
        let output = data.to_bytes().unwrap();

        assert_eq!(input, output.as_slice());
    }

    #[test]
    pub fn u8() {
        encode_decode::<u8>(&[170]);
    }

    #[test]
    pub fn u16() {
        encode_decode::<u16>(&[170, 85]);
    }

    #[test]
    pub fn u32() {
        encode_decode::<u32>(&[170, 85, 170, 85]);
    }

    #[test]
    pub fn u64() {
        encode_decode::<u64>(&[170, 85, 170, 85, 170, 85, 170, 85]);
    }

    #[test]
    pub fn i8() {
        encode_decode::<i8>(&[170]);
    }

    #[test]
    pub fn i16() {
        encode_decode::<i16>(&[170, 85]);
    }

    #[test]
    pub fn i32() {
        encode_decode::<i32>(&[170, 85, 170, 85]);
    }

    #[test]
    pub fn i64() {
        encode_decode::<i64>(&[170, 85, 170, 85, 170, 85, 170, 85]);
    }

    #[test]
    pub fn f32() {
        encode_decode::<f32>(&[170, 85, 170, 85]);
    }

    #[test]
    pub fn array() {
        encode_decode::<[u8; 4]>(&[1, 2, 3, 4]);
    }

    #[test]
    pub fn string() {
        encode_decode::<String>(b"testing\0");
    }

    #[test]
    pub fn vector() {
        encode_decode::<Vec<u8>>(&[1, 2, 3, 4]);
    }

    #[cfg(feature = "cgmath")]
    #[test]
    pub fn vector2() {
        encode_decode::<cgmath::Vector2<u8>>(&[1, 2]);
    }

    #[cfg(feature = "cgmath")]
    #[test]
    pub fn vector3() {
        encode_decode::<cgmath::Vector3<u8>>(&[1, 2, 3]);
    }

    #[cfg(feature = "cgmath")]
    #[test]
    pub fn vector4() {
        encode_decode::<cgmath::Vector4<u8>>(&[1, 2, 3, 4]);
    }

    #[cfg(feature = "cgmath")]
    #[test]
    pub fn quaternion() {
        encode_decode::<cgmath::Quaternion<u8>>(&[1, 2, 3, 4]);
    }

    #[cfg(feature = "cgmath")]
    #[test]
    pub fn matrix3() {
        encode_decode::<cgmath::Matrix3<u8>>(&[1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }
}
