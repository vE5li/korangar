#[cfg(feature = "cgmath")]
use cgmath::{Vector2, Vector3, Vector4};

use crate::{ConversionResult, ConversionResultExt, ToBytes};

impl ToBytes for u8 {
    fn to_bytes(&self) -> ConversionResult<Vec<u8>> {
        Ok(vec![*self])
    }
}

impl ToBytes for u16 {
    fn to_bytes(&self) -> ConversionResult<Vec<u8>> {
        Ok(self.to_le_bytes().to_vec())
    }
}

impl ToBytes for u32 {
    fn to_bytes(&self) -> ConversionResult<Vec<u8>> {
        Ok(self.to_le_bytes().to_vec())
    }
}

impl ToBytes for u64 {
    fn to_bytes(&self) -> ConversionResult<Vec<u8>> {
        Ok(self.to_le_bytes().to_vec())
    }
}

impl ToBytes for i8 {
    fn to_bytes(&self) -> ConversionResult<Vec<u8>> {
        Ok(vec![*self as u8])
    }
}

impl ToBytes for i16 {
    fn to_bytes(&self) -> ConversionResult<Vec<u8>> {
        Ok(self.to_le_bytes().to_vec())
    }
}

impl ToBytes for i32 {
    fn to_bytes(&self) -> ConversionResult<Vec<u8>> {
        Ok(self.to_le_bytes().to_vec())
    }
}

impl ToBytes for i64 {
    fn to_bytes(&self) -> ConversionResult<Vec<u8>> {
        Ok(self.to_le_bytes().to_vec())
    }
}

impl ToBytes for f32 {
    fn to_bytes(&self) -> ConversionResult<Vec<u8>> {
        Ok(self.to_ne_bytes().to_vec())
    }
}

impl<T: ToBytes, const SIZE: usize> ToBytes for [T; SIZE] {
    fn to_bytes(&self) -> ConversionResult<Vec<u8>> {
        let mut bytes = Vec::new();

        for item in self.iter() {
            let item = item.to_bytes().trace::<Self>()?;
            bytes.extend(item);
        }

        Ok(bytes)
    }
}

impl ToBytes for String {
    fn to_bytes(&self) -> ConversionResult<Vec<u8>> {
        Ok(self.bytes().chain(std::iter::once(0)).collect())
    }
}

#[cfg(feature = "cgmath")]
impl<T: ToBytes> ToBytes for Vector2<T> {
    fn to_bytes(&self) -> ConversionResult<Vec<u8>> {
        let mut bytes = self.x.to_bytes().trace::<Self>()?;
        bytes.append(&mut self.y.to_bytes().trace::<Self>()?);

        Ok(bytes)
    }
}

#[cfg(feature = "cgmath")]
impl<T: ToBytes> ToBytes for Vector3<T> {
    fn to_bytes(&self) -> ConversionResult<Vec<u8>> {
        let mut bytes = self.x.to_bytes().trace::<Self>()?;
        bytes.append(&mut self.y.to_bytes().trace::<Self>()?);
        bytes.append(&mut self.z.to_bytes().trace::<Self>()?);

        Ok(bytes)
    }
}

#[cfg(feature = "cgmath")]
impl<T: ToBytes> ToBytes for Vector4<T> {
    fn to_bytes(&self) -> ConversionResult<Vec<u8>> {
        let mut bytes = self.x.to_bytes().trace::<Self>()?;
        bytes.append(&mut self.y.to_bytes().trace::<Self>()?);
        bytes.append(&mut self.z.to_bytes().trace::<Self>()?);
        bytes.append(&mut self.w.to_bytes().trace::<Self>()?);
        Ok(bytes)
    }
}
