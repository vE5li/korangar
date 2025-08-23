#[cfg(feature = "cgmath")]
use cgmath::{Matrix3, Point2, Point3, Quaternion, Vector2, Vector3, Vector4};

use crate::{ByteWriter, ConversionResult, ConversionResultExt, ToBytes};

impl ToBytes for u8 {
    fn to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        byte_writer.push(*self);
        Ok(size_of::<Self>())
    }
}

impl ToBytes for u16 {
    fn to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        byte_writer.extend_from_slice(&self.to_le_bytes());
        Ok(size_of::<Self>())
    }
}

impl ToBytes for u32 {
    fn to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        byte_writer.extend_from_slice(&self.to_le_bytes());
        Ok(size_of::<Self>())
    }
}

impl ToBytes for u64 {
    fn to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        byte_writer.extend_from_slice(&self.to_le_bytes());
        Ok(size_of::<Self>())
    }
}

impl ToBytes for i8 {
    fn to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        byte_writer.extend_from_slice(&self.to_le_bytes());
        Ok(size_of::<Self>())
    }
}

impl ToBytes for i16 {
    fn to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        byte_writer.extend_from_slice(&self.to_le_bytes());
        Ok(size_of::<Self>())
    }
}

impl ToBytes for i32 {
    fn to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        byte_writer.extend_from_slice(&self.to_le_bytes());
        Ok(size_of::<Self>())
    }
}

impl ToBytes for i64 {
    fn to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        byte_writer.extend_from_slice(&self.to_le_bytes());
        Ok(size_of::<Self>())
    }
}

impl ToBytes for f32 {
    fn to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        byte_writer.extend_from_slice(&self.to_le_bytes());
        Ok(size_of::<Self>())
    }
}

impl<T: ToBytes, const SIZE: usize> ToBytes for [T; SIZE] {
    fn to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        byte_writer.write_counted(|writer| {
            for item in self.iter() {
                item.to_bytes(writer).trace::<Self>()?;
            }

            Ok(())
        })
    }
}

impl ToBytes for String {
    fn to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        byte_writer.write_counted(|writer| {
            writer.encode_string(self.as_str());

            Ok(())
        })
    }
}

impl<T: ToBytes> ToBytes for Vec<T> {
    fn to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        byte_writer.write_counted(|writer| {
            for item in self.iter() {
                item.to_bytes(writer).trace::<Self>()?;
            }

            Ok(())
        })
    }
}

#[cfg(feature = "cgmath")]
impl<T: ToBytes> ToBytes for Vector2<T> {
    fn to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        byte_writer.write_counted(|writer| {
            self.x.to_bytes(writer).trace::<Self>()?;
            self.y.to_bytes(writer).trace::<Self>()?;

            Ok(())
        })
    }
}

#[cfg(feature = "cgmath")]
impl<T: ToBytes> ToBytes for Vector3<T> {
    fn to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        byte_writer.write_counted(|writer| {
            self.x.to_bytes(writer).trace::<Self>()?;
            self.y.to_bytes(writer).trace::<Self>()?;
            self.z.to_bytes(writer).trace::<Self>()?;

            Ok(())
        })
    }
}

#[cfg(feature = "cgmath")]
impl<T: ToBytes> ToBytes for Vector4<T> {
    fn to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        byte_writer.write_counted(|writer| {
            self.x.to_bytes(writer).trace::<Self>()?;
            self.y.to_bytes(writer).trace::<Self>()?;
            self.z.to_bytes(writer).trace::<Self>()?;
            self.w.to_bytes(writer).trace::<Self>()?;

            Ok(())
        })
    }
}

#[cfg(feature = "cgmath")]
impl<T: ToBytes> ToBytes for Point2<T> {
    fn to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        byte_writer.write_counted(|writer| {
            self.x.to_bytes(writer).trace::<Self>()?;
            self.y.to_bytes(writer).trace::<Self>()?;

            Ok(())
        })
    }
}

#[cfg(feature = "cgmath")]
impl<T: ToBytes> ToBytes for Point3<T> {
    fn to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        byte_writer.write_counted(|writer| {
            self.x.to_bytes(writer).trace::<Self>()?;
            self.y.to_bytes(writer).trace::<Self>()?;
            self.z.to_bytes(writer).trace::<Self>()?;

            Ok(())
        })
    }
}

#[cfg(feature = "cgmath")]
impl<T: ToBytes> ToBytes for Quaternion<T> {
    fn to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        byte_writer.write_counted(|writer| {
            self.v.x.to_bytes(writer).trace::<Self>()?;
            self.v.y.to_bytes(writer).trace::<Self>()?;
            self.v.z.to_bytes(writer).trace::<Self>()?;
            self.s.to_bytes(writer).trace::<Self>()?;

            Ok(())
        })
    }
}

#[cfg(feature = "cgmath")]
impl<T: ToBytes> ToBytes for Matrix3<T> {
    fn to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        byte_writer.write_counted(|writer| {
            self.x.x.to_bytes(writer).trace::<Self>()?;
            self.x.y.to_bytes(writer).trace::<Self>()?;
            self.x.z.to_bytes(writer).trace::<Self>()?;

            self.y.x.to_bytes(writer).trace::<Self>()?;
            self.y.y.to_bytes(writer).trace::<Self>()?;
            self.y.z.to_bytes(writer).trace::<Self>()?;

            self.z.x.to_bytes(writer).trace::<Self>()?;
            self.z.y.to_bytes(writer).trace::<Self>()?;
            self.z.z.to_bytes(writer).trace::<Self>()?;

            Ok(())
        })
    }
}
