#[cfg(feature = "cgmath")]
use cgmath::{BaseFloat, Matrix3, Point3, Quaternion, Vector2, Vector3, Vector4};

use crate::{ByteStream, ConversionResult, ConversionResultExt, FromBytes};

impl FromBytes for u8 {
    fn from_bytes<Meta>(byte_stream: &mut ByteStream<Meta>) -> ConversionResult<Self> {
        byte_stream.byte::<Self>()
    }
}

impl FromBytes for u16 {
    fn from_bytes<Meta>(byte_stream: &mut ByteStream<Meta>) -> ConversionResult<Self> {
        let array: Result<[u8; 2], _> = core::array::try_from_fn(|_| byte_stream.byte::<Self>());
        Ok(Self::from_le_bytes(array?))
    }
}

impl FromBytes for u32 {
    fn from_bytes<Meta>(byte_stream: &mut ByteStream<Meta>) -> ConversionResult<Self> {
        let array: Result<[u8; 4], _> = core::array::try_from_fn(|_| byte_stream.byte::<Self>());
        Ok(Self::from_le_bytes(array?))
    }
}

impl FromBytes for u64 {
    fn from_bytes<Meta>(byte_stream: &mut ByteStream<Meta>) -> ConversionResult<Self> {
        let array: Result<[u8; 8], _> = core::array::try_from_fn(|_| byte_stream.byte::<Self>());
        Ok(Self::from_le_bytes(array?))
    }
}

impl FromBytes for i8 {
    fn from_bytes<Meta>(byte_stream: &mut ByteStream<Meta>) -> ConversionResult<Self> {
        Ok(byte_stream.byte::<Self>()? as i8)
    }
}

impl FromBytes for i16 {
    fn from_bytes<Meta>(byte_stream: &mut ByteStream<Meta>) -> ConversionResult<Self> {
        let array: Result<[u8; 2], _> = core::array::try_from_fn(|_| byte_stream.byte::<Self>());
        Ok(Self::from_le_bytes(array?))
    }
}

impl FromBytes for i32 {
    fn from_bytes<Meta>(byte_stream: &mut ByteStream<Meta>) -> ConversionResult<Self> {
        let array: Result<[u8; 4], _> = core::array::try_from_fn(|_| byte_stream.byte::<Self>());
        Ok(Self::from_le_bytes(array?))
    }
}

impl FromBytes for i64 {
    fn from_bytes<Meta>(byte_stream: &mut ByteStream<Meta>) -> ConversionResult<Self> {
        let array: Result<[u8; 8], _> = core::array::try_from_fn(|_| byte_stream.byte::<Self>());
        Ok(Self::from_le_bytes(array?))
    }
}

impl FromBytes for f32 {
    fn from_bytes<Meta>(byte_stream: &mut ByteStream<Meta>) -> ConversionResult<Self> {
        let array: Result<[u8; 4], _> = core::array::try_from_fn(|_| byte_stream.byte::<Self>());
        Ok(Self::from_le_bytes(array?))
    }
}

impl<T: FromBytes, const SIZE: usize> FromBytes for [T; SIZE] {
    fn from_bytes<Meta>(byte_stream: &mut ByteStream<Meta>) -> ConversionResult<Self> {
        let array: Result<[T; SIZE], _> = core::array::try_from_fn(|_| T::from_bytes(byte_stream).trace::<Self>());
        Ok(array?)
    }
}

impl FromBytes for String {
    fn from_bytes<Meta>(byte_stream: &mut ByteStream<Meta>) -> ConversionResult<Self> {
        let mut value = String::new();

        while let Ok(byte) = byte_stream.byte::<Self>() {
            match byte {
                0 => break,
                byte => value.push(byte as char),
            }
        }

        Ok(value)
    }
}

impl<T: FromBytes> FromBytes for Vec<T> {
    fn from_bytes<Meta>(byte_stream: &mut ByteStream<Meta>) -> ConversionResult<Self> {
        let mut vector = Vec::new();

        while !byte_stream.is_empty() {
            let item = T::from_bytes(byte_stream).trace::<Self>()?;
            vector.push(item);
        }

        Ok(vector)
    }
}

#[cfg(feature = "cgmath")]
impl<T: FromBytes + Clone> FromBytes for Vector2<T> {
    fn from_bytes<Meta>(byte_stream: &mut ByteStream<Meta>) -> ConversionResult<Self> {
        let array: Result<[T; 2], _> = core::array::try_from_fn(|_| T::from_bytes(byte_stream).trace::<Self>());
        Ok(Vector2::<T>::from(array?))
    }
}

#[cfg(feature = "cgmath")]
impl<T: FromBytes + Clone> FromBytes for Vector3<T> {
    fn from_bytes<Meta>(byte_stream: &mut ByteStream<Meta>) -> ConversionResult<Self> {
        let array: Result<[T; 3], _> = core::array::try_from_fn(|_| T::from_bytes(byte_stream).trace::<Self>());
        Ok(Vector3::<T>::from(array?))
    }
}

#[cfg(feature = "cgmath")]
impl<T: FromBytes + Clone> FromBytes for Vector4<T> {
    fn from_bytes<Meta>(byte_stream: &mut ByteStream<Meta>) -> ConversionResult<Self> {
        let array: Result<[T; 4], _> = core::array::try_from_fn(|_| T::from_bytes(byte_stream).trace::<Self>());
        Ok(Vector4::<T>::from(array?))
    }
}

#[cfg(feature = "cgmath")]
impl<T: FromBytes + Clone> FromBytes for Point3<T> {
    fn from_bytes<Meta>(byte_stream: &mut ByteStream<Meta>) -> ConversionResult<Self> {
        let array: Result<[T; 3], _> = core::array::try_from_fn(|_| T::from_bytes(byte_stream).trace::<Self>());
        Ok(Point3::<T>::from(array?))
    }
}

#[cfg(feature = "cgmath")]
impl<T: FromBytes + BaseFloat> FromBytes for Quaternion<T> {
    fn from_bytes<Meta>(byte_stream: &mut ByteStream<Meta>) -> ConversionResult<Self> {
        let array: Result<[T; 4], _> = core::array::try_from_fn(|_| T::from_bytes(byte_stream).trace::<Self>());
        Ok(Quaternion::<T>::from(array?))
    }
}

#[cfg(feature = "cgmath")]
impl<T: FromBytes + Clone> FromBytes for Matrix3<T> {
    fn from_bytes<Meta>(byte_stream: &mut ByteStream<Meta>) -> ConversionResult<Self> {
        let array_0: Result<[T; 3], _> = core::array::try_from_fn(|_| T::from_bytes(byte_stream).trace::<Self>());
        let column_0 = Vector3::<T>::from(array_0?);

        let array_1: Result<[T; 3], _> = core::array::try_from_fn(|_| T::from_bytes(byte_stream).trace::<Self>());
        let column_1 = Vector3::<T>::from(array_1?);

        let array_2: Result<[T; 3], _> = core::array::try_from_fn(|_| T::from_bytes(byte_stream).trace::<Self>());
        let column_2 = Vector3::<T>::from(array_2?);

        Ok(Matrix3::from_cols(column_0, column_1, column_2))
    }
}
