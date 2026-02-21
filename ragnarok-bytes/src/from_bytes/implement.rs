#[cfg(feature = "cgmath")]
use cgmath::{Matrix3, Point2, Point3, Quaternion, Vector2, Vector3, Vector4};

use crate::{ByteReader, ConversionResult, ConversionResultExt, FromBytes};

impl FromBytes for u8 {
    fn from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self> {
        byte_reader.byte::<Self>()
    }
}

impl FromBytes for u16 {
    fn from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self> {
        Ok(Self::from_le_bytes(byte_reader.bytes::<Self, 2>()?))
    }
}

impl FromBytes for u32 {
    fn from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self> {
        Ok(Self::from_le_bytes(byte_reader.bytes::<Self, 4>()?))
    }
}

impl FromBytes for u64 {
    fn from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self> {
        Ok(Self::from_le_bytes(byte_reader.bytes::<Self, 8>()?))
    }
}

impl FromBytes for i8 {
    fn from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self> {
        Ok(byte_reader.byte::<Self>()? as i8)
    }
}

impl FromBytes for i16 {
    fn from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self> {
        Ok(Self::from_le_bytes(byte_reader.bytes::<Self, 2>()?))
    }
}

impl FromBytes for i32 {
    fn from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self> {
        Ok(Self::from_le_bytes(byte_reader.bytes::<Self, 4>()?))
    }
}

impl FromBytes for i64 {
    fn from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self> {
        Ok(Self::from_le_bytes(byte_reader.bytes::<Self, 8>()?))
    }
}

impl FromBytes for f32 {
    fn from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self> {
        Ok(Self::from_le_bytes(byte_reader.bytes::<Self, 4>()?))
    }
}

impl<T: FromBytes, const SIZE: usize> FromBytes for [T; SIZE] {
    fn from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self> {
        std::array::try_from_fn(|_| T::from_bytes(byte_reader)).trace::<Self>()
    }
}

impl FromBytes for String {
    fn from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self> {
        let mut bytes = Vec::<u8>::new();

        while let Ok(byte) = byte_reader.byte::<Self>() {
            match byte {
                0 => break,
                byte => bytes.push(byte),
            }
        }

        Ok(byte_reader.decode_string(&bytes))
    }
}

impl<T: FromBytes> FromBytes for Vec<T> {
    fn from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self> {
        let mut vector = Vec::new();

        while !byte_reader.is_empty() {
            let item = T::from_bytes(byte_reader).trace::<Self>()?;
            vector.push(item);
        }

        Ok(vector)
    }
}

#[cfg(feature = "cgmath")]
impl<T: FromBytes> FromBytes for Vector2<T> {
    fn from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self> {
        let first = T::from_bytes(byte_reader).trace::<Self>()?;
        let second = T::from_bytes(byte_reader).trace::<Self>()?;

        Ok(Vector2::new(first, second))
    }
}

#[cfg(feature = "cgmath")]
impl<T: FromBytes> FromBytes for Vector3<T> {
    fn from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self> {
        let first = T::from_bytes(byte_reader).trace::<Self>()?;
        let second = T::from_bytes(byte_reader).trace::<Self>()?;
        let third = T::from_bytes(byte_reader).trace::<Self>()?;

        Ok(Vector3::new(first, second, third))
    }
}

#[cfg(feature = "cgmath")]
impl<T: FromBytes> FromBytes for Vector4<T> {
    fn from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self> {
        let first = T::from_bytes(byte_reader).trace::<Self>()?;
        let second = T::from_bytes(byte_reader).trace::<Self>()?;
        let third = T::from_bytes(byte_reader).trace::<Self>()?;
        let fourth = T::from_bytes(byte_reader).trace::<Self>()?;

        Ok(Vector4::new(first, second, third, fourth))
    }
}

#[cfg(feature = "cgmath")]
impl<T: FromBytes> FromBytes for Point2<T> {
    fn from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self> {
        let first = T::from_bytes(byte_reader).trace::<Self>()?;
        let second = T::from_bytes(byte_reader).trace::<Self>()?;

        Ok(Point2::new(first, second))
    }
}

#[cfg(feature = "cgmath")]
impl<T: FromBytes> FromBytes for Point3<T> {
    fn from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self> {
        let first = T::from_bytes(byte_reader).trace::<Self>()?;
        let second = T::from_bytes(byte_reader).trace::<Self>()?;
        let third = T::from_bytes(byte_reader).trace::<Self>()?;

        Ok(Point3::new(first, second, third))
    }
}

#[cfg(feature = "cgmath")]
impl<T: FromBytes> FromBytes for Quaternion<T> {
    fn from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self> {
        let first = T::from_bytes(byte_reader).trace::<Self>()?;
        let second = T::from_bytes(byte_reader).trace::<Self>()?;
        let third = T::from_bytes(byte_reader).trace::<Self>()?;
        let fourth = T::from_bytes(byte_reader).trace::<Self>()?;

        Ok(Quaternion::new(fourth, first, second, third))
    }
}

#[cfg(feature = "cgmath")]
impl<T: FromBytes> FromBytes for Matrix3<T> {
    fn from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self> {
        let c0r0 = T::from_bytes(byte_reader).trace::<Self>()?;
        let c0r1 = T::from_bytes(byte_reader).trace::<Self>()?;
        let c0r2 = T::from_bytes(byte_reader).trace::<Self>()?;

        let c1r0 = T::from_bytes(byte_reader).trace::<Self>()?;
        let c1r1 = T::from_bytes(byte_reader).trace::<Self>()?;
        let c1r2 = T::from_bytes(byte_reader).trace::<Self>()?;

        let c2r0 = T::from_bytes(byte_reader).trace::<Self>()?;
        let c2r1 = T::from_bytes(byte_reader).trace::<Self>()?;
        let c2r2 = T::from_bytes(byte_reader).trace::<Self>()?;

        Ok(Matrix3::new(c0r0, c0r1, c0r2, c1r0, c1r1, c1r2, c2r0, c2r1, c2r2))
    }
}
