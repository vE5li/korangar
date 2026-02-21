#![feature(array_try_from_fn)]

mod error;
mod fixed;
mod from_bytes;
mod metadata;
mod reader;
mod to_bytes;
mod writer;

pub use encoding_rs as encoding;
#[cfg(feature = "derive")]
pub use ragnarok_macros::{ByteConvertable, FixedByteSize, FromBytes, ToBytes};

pub use self::error::{ConversionError, ConversionErrorType, ConversionResult, ConversionResultExt};
pub use self::fixed::{FixedByteSize, FixedByteSizeCollection};
pub use self::from_bytes::{FromBytes, FromBytesExt};
pub use self::metadata::{CastableMetadata, Caster, DynMetadata};
pub use self::reader::ByteReader;
pub use self::to_bytes::{ToBytes, ToBytesExt};
pub use self::writer::ByteWriter;

#[cfg(test)]
mod conversion {
    use crate::from_bytes::FromBytesExt;
    use crate::{ByteReader, ByteWriter, FromBytes, ToBytes};

    fn encode_decode<T: FromBytes + ToBytes>(input: &[u8]) {
        let mut byte_reader = ByteReader::without_metadata(input);

        let data = T::from_bytes(&mut byte_reader).unwrap();

        let mut byte_writer = ByteWriter::new();
        data.to_bytes(&mut byte_writer).unwrap();
        let bytes = byte_writer.into_inner();

        assert_eq!(input, bytes.as_slice());
    }

    #[test]
    fn u8() {
        encode_decode::<u8>(&[170]);
    }

    #[test]
    fn u16() {
        encode_decode::<u16>(&[170, 85]);
    }

    #[test]
    fn u32() {
        encode_decode::<u32>(&[170, 85, 170, 85]);
    }

    #[test]
    fn u64() {
        encode_decode::<u64>(&[170, 85, 170, 85, 170, 85, 170, 85]);
    }

    #[test]
    fn i8() {
        encode_decode::<i8>(&[170]);
    }

    #[test]
    fn i16() {
        encode_decode::<i16>(&[170, 85]);
    }

    #[test]
    fn i32() {
        encode_decode::<i32>(&[170, 85, 170, 85]);
    }

    #[test]
    fn i64() {
        encode_decode::<i64>(&[170, 85, 170, 85, 170, 85, 170, 85]);
    }

    #[test]
    fn f32() {
        encode_decode::<f32>(&[170, 85, 170, 85]);
    }

    #[test]
    fn array() {
        encode_decode::<[u8; 4]>(&[1, 2, 3, 4]);
    }

    #[test]
    fn string() {
        encode_decode::<String>(b"testing\0");
    }

    #[test]
    fn vector() {
        encode_decode::<Vec<u8>>(&[1, 2, 3, 4]);
    }

    #[cfg(feature = "cgmath")]
    #[test]
    fn vector2() {
        encode_decode::<cgmath::Vector2<u8>>(&[1, 2]);
    }

    #[cfg(feature = "cgmath")]
    #[test]
    fn vector3() {
        encode_decode::<cgmath::Vector3<u8>>(&[1, 2, 3]);
    }

    #[cfg(feature = "cgmath")]
    #[test]
    fn vector4() {
        encode_decode::<cgmath::Vector4<u8>>(&[1, 2, 3, 4]);
    }

    #[cfg(feature = "cgmath")]
    #[test]
    fn quaternion() {
        encode_decode::<cgmath::Quaternion<u8>>(&[1, 2, 3, 4]);
    }

    #[cfg(feature = "cgmath")]
    #[test]
    fn matrix3() {
        encode_decode::<cgmath::Matrix3<u8>>(&[1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }

    #[test]
    fn full_lenght_string() {
        let mut byte_stream = ByteReader::without_metadata(&[65, 65, 65, 65]);

        assert_eq!(String::from_n_bytes(&mut byte_stream, 4).unwrap(), "AAAA")
    }
}
