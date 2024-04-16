#[cfg(feature = "cgmath")]
use cgmath::{Matrix3, Quaternion, Vector2, Vector3, Vector4};

use crate::{ByteStream, ConversionResult, ConversionResultExt, FromBytes};

impl FromBytes for u8 {
    fn from_bytes<META>(byte_stream: &mut ByteStream<META>) -> ConversionResult<Self> {
        byte_stream.byte::<Self>()
    }
}

impl FromBytes for u16 {
    fn from_bytes<META>(byte_stream: &mut ByteStream<META>) -> ConversionResult<Self> {
        Ok(Self::from_le_bytes([byte_stream.byte::<Self>()?, byte_stream.byte::<Self>()?]))
    }
}

impl FromBytes for u32 {
    fn from_bytes<META>(byte_stream: &mut ByteStream<META>) -> ConversionResult<Self> {
        Ok(Self::from_le_bytes([
            byte_stream.byte::<Self>()?,
            byte_stream.byte::<Self>()?,
            byte_stream.byte::<Self>()?,
            byte_stream.byte::<Self>()?,
        ]))
    }
}

impl FromBytes for u64 {
    fn from_bytes<META>(byte_stream: &mut ByteStream<META>) -> ConversionResult<Self> {
        Ok(Self::from_le_bytes([
            byte_stream.byte::<Self>()?,
            byte_stream.byte::<Self>()?,
            byte_stream.byte::<Self>()?,
            byte_stream.byte::<Self>()?,
            byte_stream.byte::<Self>()?,
            byte_stream.byte::<Self>()?,
            byte_stream.byte::<Self>()?,
            byte_stream.byte::<Self>()?,
        ]))
    }
}

impl FromBytes for i8 {
    fn from_bytes<META>(byte_stream: &mut ByteStream<META>) -> ConversionResult<Self> {
        Ok(byte_stream.byte::<Self>()? as i8)
    }
}

impl FromBytes for i16 {
    fn from_bytes<META>(byte_stream: &mut ByteStream<META>) -> ConversionResult<Self> {
        Ok(Self::from_le_bytes([byte_stream.byte::<Self>()?, byte_stream.byte::<Self>()?]))
    }
}

impl FromBytes for i32 {
    fn from_bytes<META>(byte_stream: &mut ByteStream<META>) -> ConversionResult<Self> {
        Ok(Self::from_le_bytes([
            byte_stream.byte::<Self>()?,
            byte_stream.byte::<Self>()?,
            byte_stream.byte::<Self>()?,
            byte_stream.byte::<Self>()?,
        ]))
    }
}

impl FromBytes for i64 {
    fn from_bytes<META>(byte_stream: &mut ByteStream<META>) -> ConversionResult<Self> {
        Ok(Self::from_le_bytes([
            byte_stream.byte::<Self>()?,
            byte_stream.byte::<Self>()?,
            byte_stream.byte::<Self>()?,
            byte_stream.byte::<Self>()?,
            byte_stream.byte::<Self>()?,
            byte_stream.byte::<Self>()?,
            byte_stream.byte::<Self>()?,
            byte_stream.byte::<Self>()?,
        ]))
    }
}

impl FromBytes for f32 {
    fn from_bytes<META>(byte_stream: &mut ByteStream<META>) -> ConversionResult<Self> {
        Ok(Self::from_le_bytes([
            byte_stream.byte::<Self>()?,
            byte_stream.byte::<Self>()?,
            byte_stream.byte::<Self>()?,
            byte_stream.byte::<Self>()?,
        ]))
    }
}

impl<T: FromBytes, const SIZE: usize> FromBytes for [T; SIZE] {
    fn from_bytes<META>(byte_stream: &mut ByteStream<META>) -> ConversionResult<Self> {
        use std::mem::MaybeUninit;

        let mut data: [MaybeUninit<T>; SIZE] = unsafe { MaybeUninit::uninit().assume_init() };

        for element in &mut data[..] {
            let item = T::from_bytes(byte_stream).trace::<Self>()?;
            *element = MaybeUninit::new(item);
        }

        // rust wont let us do this currently
        //unsafe { mem::transmute::<_, [T; SIZE]>(data) }

        // workaround from: https://github.com/rust-lang/rust/issues/61956
        let ptr = &mut data as *mut _ as *mut [T; SIZE];
        let result = unsafe { ptr.read() };

        core::mem::forget(data);

        Ok(result)
    }
}

impl FromBytes for String {
    fn from_bytes<META>(byte_stream: &mut ByteStream<META>) -> ConversionResult<Self> {
        let mut value = String::new();

        loop {
            match byte_stream.byte::<Self>()? {
                0 => break,
                byte => value.push(byte as char),
            }
        }

        Ok(value)
    }
}

impl<T: FromBytes> FromBytes for Vec<T> {
    fn from_bytes<META>(byte_stream: &mut ByteStream<META>) -> ConversionResult<Self> {
        let mut vector = Vec::new();

        while !byte_stream.is_empty() {
            let item = T::from_bytes(byte_stream).trace::<Self>()?;
            vector.push(item);
        }

        Ok(vector)
    }
}

#[cfg(feature = "cgmath")]
impl<T: FromBytes> FromBytes for Vector2<T> {
    fn from_bytes<META>(byte_stream: &mut ByteStream<META>) -> ConversionResult<Self> {
        let first = T::from_bytes(byte_stream).trace::<Self>()?;
        let second = T::from_bytes(byte_stream).trace::<Self>()?;

        Ok(Vector2::new(first, second))
    }
}

#[cfg(feature = "cgmath")]
impl<T: FromBytes> FromBytes for Vector3<T> {
    fn from_bytes<META>(byte_stream: &mut ByteStream<META>) -> ConversionResult<Self> {
        let first = T::from_bytes(byte_stream).trace::<Self>()?;
        let second = T::from_bytes(byte_stream).trace::<Self>()?;
        let third = T::from_bytes(byte_stream).trace::<Self>()?;

        Ok(Vector3::new(first, second, third))
    }
}

#[cfg(feature = "cgmath")]
impl<T: FromBytes> FromBytes for Vector4<T> {
    fn from_bytes<META>(byte_stream: &mut ByteStream<META>) -> ConversionResult<Self> {
        let first = T::from_bytes(byte_stream).trace::<Self>()?;
        let second = T::from_bytes(byte_stream).trace::<Self>()?;
        let third = T::from_bytes(byte_stream).trace::<Self>()?;
        let fourth = T::from_bytes(byte_stream).trace::<Self>()?;

        Ok(Vector4::new(first, second, third, fourth))
    }
}

#[cfg(feature = "cgmath")]
impl<T: FromBytes> FromBytes for Quaternion<T> {
    fn from_bytes<META>(byte_stream: &mut ByteStream<META>) -> ConversionResult<Self> {
        let first = T::from_bytes(byte_stream).trace::<Self>()?;
        let second = T::from_bytes(byte_stream).trace::<Self>()?;
        let third = T::from_bytes(byte_stream).trace::<Self>()?;
        let fourth = T::from_bytes(byte_stream).trace::<Self>()?;

        Ok(Quaternion::new(fourth, first, second, third))
    }
}

#[cfg(feature = "cgmath")]
impl<T: FromBytes> FromBytes for Matrix3<T> {
    fn from_bytes<META>(byte_stream: &mut ByteStream<META>) -> ConversionResult<Self> {
        let c0r0 = T::from_bytes(byte_stream).trace::<Self>()?;
        let c0r1 = T::from_bytes(byte_stream).trace::<Self>()?;
        let c0r2 = T::from_bytes(byte_stream).trace::<Self>()?;

        let c1r0 = T::from_bytes(byte_stream).trace::<Self>()?;
        let c1r1 = T::from_bytes(byte_stream).trace::<Self>()?;
        let c1r2 = T::from_bytes(byte_stream).trace::<Self>()?;

        let c2r0 = T::from_bytes(byte_stream).trace::<Self>()?;
        let c2r1 = T::from_bytes(byte_stream).trace::<Self>()?;
        let c2r2 = T::from_bytes(byte_stream).trace::<Self>()?;

        Ok(Matrix3::new(c0r0, c0r1, c0r2, c1r0, c1r1, c1r2, c2r0, c2r1, c2r2))
    }
}
