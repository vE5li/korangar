use cgmath::{Matrix3, Quaternion, Vector2, Vector3, Vector4};

use crate::loaders::ByteStream;

mod error;
mod helper;
mod named;

pub use self::error::{ConversionError, ConversionErrorType};
pub use self::helper::*;
pub use self::named::Named;

pub trait FromBytes: Named {
    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Result<Self, Box<ConversionError>>
    where
        Self: Sized;
}

pub trait ToBytes: Named {
    fn to_bytes(&self, length_hint: Option<usize>) -> Result<Vec<u8>, Box<ConversionError>>;
}

impl Named for u8 {
    const NAME: &'static str = "u8";
}

impl FromBytes for u8 {
    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Result<Self, Box<ConversionError>> {
        check_length_hint_none::<Self>(length_hint)?;
        byte_stream.next::<Self>()
    }
}

impl ToBytes for u8 {
    fn to_bytes(&self, length_hint: Option<usize>) -> Result<Vec<u8>, Box<ConversionError>> {
        check_length_hint_none::<Self>(length_hint)?;
        Ok(vec![*self])
    }
}

impl Named for u16 {
    const NAME: &'static str = "u16";
}

impl FromBytes for u16 {
    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Result<Self, Box<ConversionError>> {
        check_length_hint_none::<Self>(length_hint)?;
        Ok(Self::from_le_bytes([byte_stream.next::<Self>()?, byte_stream.next::<Self>()?]))
    }
}

impl ToBytes for u16 {
    fn to_bytes(&self, length_hint: Option<usize>) -> Result<Vec<u8>, Box<ConversionError>> {
        check_length_hint_none::<Self>(length_hint)?;
        Ok(self.to_le_bytes().to_vec())
    }
}

impl Named for u32 {
    const NAME: &'static str = "u32";
}

impl FromBytes for u32 {
    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Result<Self, Box<ConversionError>> {
        check_length_hint_none::<Self>(length_hint)?;
        Ok(Self::from_le_bytes([
            byte_stream.next::<Self>()?,
            byte_stream.next::<Self>()?,
            byte_stream.next::<Self>()?,
            byte_stream.next::<Self>()?,
        ]))
    }
}

impl ToBytes for u32 {
    fn to_bytes(&self, length_hint: Option<usize>) -> Result<Vec<u8>, Box<ConversionError>> {
        check_length_hint_none::<Self>(length_hint)?;
        Ok(self.to_le_bytes().to_vec())
    }
}

impl Named for u64 {
    const NAME: &'static str = "u64";
}

impl FromBytes for u64 {
    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Result<Self, Box<ConversionError>> {
        check_length_hint_none::<Self>(length_hint)?;
        Ok(Self::from_le_bytes([
            byte_stream.next::<Self>()?,
            byte_stream.next::<Self>()?,
            byte_stream.next::<Self>()?,
            byte_stream.next::<Self>()?,
            byte_stream.next::<Self>()?,
            byte_stream.next::<Self>()?,
            byte_stream.next::<Self>()?,
            byte_stream.next::<Self>()?,
        ]))
    }
}

impl ToBytes for u64 {
    fn to_bytes(&self, length_hint: Option<usize>) -> Result<Vec<u8>, Box<ConversionError>> {
        check_length_hint_none::<Self>(length_hint)?;
        Ok(self.to_le_bytes().to_vec())
    }
}

impl Named for i8 {
    const NAME: &'static str = "i8";
}

impl FromBytes for i8 {
    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Result<Self, Box<ConversionError>> {
        check_length_hint_none::<Self>(length_hint)?;
        Ok(byte_stream.next::<Self>()? as i8)
    }
}

impl ToBytes for i8 {
    fn to_bytes(&self, length_hint: Option<usize>) -> Result<Vec<u8>, Box<ConversionError>> {
        check_length_hint_none::<Self>(length_hint)?;
        Ok(vec![*self as u8])
    }
}

impl Named for i16 {
    const NAME: &'static str = "i16";
}

impl FromBytes for i16 {
    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Result<Self, Box<ConversionError>> {
        check_length_hint_none::<Self>(length_hint)?;
        Ok(Self::from_le_bytes([byte_stream.next::<Self>()?, byte_stream.next::<Self>()?]))
    }
}

impl ToBytes for i16 {
    fn to_bytes(&self, length_hint: Option<usize>) -> Result<Vec<u8>, Box<ConversionError>> {
        check_length_hint_none::<Self>(length_hint)?;
        Ok(self.to_le_bytes().to_vec())
    }
}

impl Named for i32 {
    const NAME: &'static str = "i32";
}

impl FromBytes for i32 {
    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Result<Self, Box<ConversionError>> {
        check_length_hint_none::<Self>(length_hint)?;
        Ok(Self::from_le_bytes([
            byte_stream.next::<Self>()?,
            byte_stream.next::<Self>()?,
            byte_stream.next::<Self>()?,
            byte_stream.next::<Self>()?,
        ]))
    }
}

impl ToBytes for i32 {
    fn to_bytes(&self, length_hint: Option<usize>) -> Result<Vec<u8>, Box<ConversionError>> {
        check_length_hint_none::<Self>(length_hint)?;
        Ok(self.to_le_bytes().to_vec())
    }
}

impl Named for i64 {
    const NAME: &'static str = "i64";
}

impl FromBytes for i64 {
    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Result<Self, Box<ConversionError>> {
        check_length_hint_none::<Self>(length_hint)?;
        Ok(Self::from_le_bytes([
            byte_stream.next::<Self>()?,
            byte_stream.next::<Self>()?,
            byte_stream.next::<Self>()?,
            byte_stream.next::<Self>()?,
            byte_stream.next::<Self>()?,
            byte_stream.next::<Self>()?,
            byte_stream.next::<Self>()?,
            byte_stream.next::<Self>()?,
        ]))
    }
}

impl ToBytes for i64 {
    fn to_bytes(&self, length_hint: Option<usize>) -> Result<Vec<u8>, Box<ConversionError>> {
        check_length_hint_none::<Self>(length_hint)?;
        Ok(self.to_le_bytes().to_vec())
    }
}

impl Named for f32 {
    const NAME: &'static str = "f32";
}

impl FromBytes for f32 {
    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Result<Self, Box<ConversionError>> {
        check_length_hint_none::<Self>(length_hint)?;
        Ok(Self::from_le_bytes([
            byte_stream.next::<Self>()?,
            byte_stream.next::<Self>()?,
            byte_stream.next::<Self>()?,
            byte_stream.next::<Self>()?,
        ]))
    }
}

impl ToBytes for f32 {
    fn to_bytes(&self, length_hint: Option<usize>) -> Result<Vec<u8>, Box<ConversionError>> {
        check_length_hint_none::<Self>(length_hint)?;
        Ok(self.to_ne_bytes().to_vec())
    }
}

impl<T: Named, const SIZE: usize> Named for [T; SIZE] {
    const NAME: &'static str = "[T; SIZE]";
}

impl<T: FromBytes, const SIZE: usize> FromBytes for [T; SIZE] {
    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Result<Self, Box<ConversionError>> {
        use std::mem::MaybeUninit;

        check_length_hint_none::<Self>(length_hint)?;

        let mut data: [MaybeUninit<T>; SIZE] = unsafe { MaybeUninit::uninit().assume_init() };

        for element in &mut data[..] {
            let foo = conversion_result::<Self, _>(T::from_bytes(byte_stream, None))?;
            *element = MaybeUninit::new(foo);
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

impl<T: ToBytes, const SIZE: usize> ToBytes for [T; SIZE] {
    fn to_bytes(&self, length_hint: Option<usize>) -> Result<Vec<u8>, Box<ConversionError>> {
        check_length_hint_none::<Self>(length_hint)?;

        let mut bytes = Vec::new();

        for item in self.iter() {
            let foo = conversion_result::<Self, _>(item.to_bytes(None))?;
            bytes.extend(foo);
        }

        Ok(bytes)
    }
}

impl Named for String {
    const NAME: &'static str = "String";
}

impl FromBytes for String {
    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Result<Self, Box<ConversionError>> {
        let mut value = String::new();
        let mut offset = 0;

        loop {
            offset += 1;

            match byte_stream.next::<Self>()? {
                0 => break,
                byte => value.push(byte as char),
            }
        }

        if let Some(length) = length_hint {
            byte_stream.skip(length - offset);
        }

        Ok(value)
    }
}

impl ToBytes for String {
    fn to_bytes(&self, length_hint: Option<usize>) -> Result<Vec<u8>, Box<ConversionError>> {
        use std::iter;

        match length_hint {
            Some(length) => {
                assert!(self.len() <= length, "string is to long for the byte stream");
                let padding = (0..length - self.len()).map(|_| 0);
                Ok(self.bytes().chain(padding).collect())
            }
            None => Ok(self.bytes().chain(iter::once(0)).collect()),
        }
    }
}

impl<T: Named> Named for Vec<T> {
    const NAME: &'static str = "Vec<T>";
}

impl<T: FromBytes> FromBytes for Vec<T> {
    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Result<Self, Box<ConversionError>> {
        let length = check_length_hint::<Self>(length_hint)?;

        let data = byte_stream.slice::<Self>(length)?;
        let mut byte_stream = ByteStream::new(data);
        let mut vector = Vec::new();

        while !byte_stream.is_empty() {
            let foo = conversion_result::<Self, _>(T::from_bytes(&mut byte_stream, None))?;
            vector.push(foo);
        }

        Ok(vector)
    }
}

impl<T: Named> Named for Vector2<T> {
    const NAME: &'static str = "Vector2<T>";
}

impl<T: FromBytes> FromBytes for Vector2<T> {
    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Result<Self, Box<ConversionError>> {
        check_length_hint_none::<Self>(length_hint)?;

        let first = conversion_result::<Self, _>(T::from_bytes(byte_stream, None))?;
        let second = conversion_result::<Self, _>(T::from_bytes(byte_stream, None))?;

        Ok(Vector2::new(first, second))
    }
}

impl<T: ToBytes> ToBytes for Vector2<T> {
    fn to_bytes(&self, length_hint: Option<usize>) -> Result<Vec<u8>, Box<ConversionError>> {
        check_length_hint_none::<Self>(length_hint)?;

        let mut bytes = conversion_result::<Self, _>(self.x.to_bytes(None))?;
        bytes.append(&mut conversion_result::<Self, _>(self.y.to_bytes(None))?);

        Ok(bytes)
    }
}

impl<T: Named> Named for Vector3<T> {
    const NAME: &'static str = "Vector3<T>";
}

impl<T: FromBytes> FromBytes for Vector3<T> {
    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Result<Self, Box<ConversionError>> {
        check_length_hint_none::<Self>(length_hint)?;

        let first = conversion_result::<Self, _>(T::from_bytes(byte_stream, None))?;
        let second = conversion_result::<Self, _>(T::from_bytes(byte_stream, None))?;
        let third = conversion_result::<Self, _>(T::from_bytes(byte_stream, None))?;

        Ok(Vector3::new(first, second, third))
    }
}

impl<T: ToBytes> ToBytes for Vector3<T> {
    fn to_bytes(&self, length_hint: Option<usize>) -> Result<Vec<u8>, Box<ConversionError>> {
        check_length_hint_none::<Self>(length_hint)?;

        let mut bytes = conversion_result::<Self, _>(self.x.to_bytes(None))?;
        bytes.append(&mut conversion_result::<Self, _>(self.y.to_bytes(None))?);
        bytes.append(&mut conversion_result::<Self, _>(self.z.to_bytes(None))?);

        Ok(bytes)
    }
}

impl<T: Named> Named for Vector4<T> {
    const NAME: &'static str = "Vector4<T>";
}

impl<T: FromBytes> FromBytes for Vector4<T> {
    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Result<Self, Box<ConversionError>> {
        check_length_hint_none::<Self>(length_hint)?;

        let first = conversion_result::<Self, _>(T::from_bytes(byte_stream, None))?;
        let second = conversion_result::<Self, _>(T::from_bytes(byte_stream, None))?;
        let third = conversion_result::<Self, _>(T::from_bytes(byte_stream, None))?;
        let fourth = conversion_result::<Self, _>(T::from_bytes(byte_stream, None))?;

        Ok(Vector4::new(first, second, third, fourth))
    }
}

impl<T: ToBytes> ToBytes for Vector4<T> {
    fn to_bytes(&self, length_hint: Option<usize>) -> Result<Vec<u8>, Box<ConversionError>> {
        check_length_hint_none::<Self>(length_hint)?;

        let mut bytes = conversion_result::<Self, _>(self.x.to_bytes(None))?;
        bytes.append(&mut conversion_result::<Self, _>(self.y.to_bytes(None))?);
        bytes.append(&mut conversion_result::<Self, _>(self.z.to_bytes(None))?);
        bytes.append(&mut conversion_result::<Self, _>(self.w.to_bytes(None))?);
        Ok(bytes)
    }
}

impl<T: Named> Named for Quaternion<T> {
    const NAME: &'static str = "Quaternion<T>";
}

impl<T: FromBytes> FromBytes for Quaternion<T> {
    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Result<Self, Box<ConversionError>> {
        check_length_hint_none::<Self>(length_hint)?;

        let first = conversion_result::<Self, _>(T::from_bytes(byte_stream, None))?;
        let second = conversion_result::<Self, _>(T::from_bytes(byte_stream, None))?;
        let third = conversion_result::<Self, _>(T::from_bytes(byte_stream, None))?;
        let fourth = conversion_result::<Self, _>(T::from_bytes(byte_stream, None))?;

        Ok(Quaternion::new(fourth, first, second, third))
    }
}

impl<T: Named> Named for Matrix3<T> {
    const NAME: &'static str = "Matrix3<T>";
}

impl<T: FromBytes> FromBytes for Matrix3<T> {
    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Result<Self, Box<ConversionError>> {
        check_length_hint_none::<Self>(length_hint)?;

        let c0r0 = conversion_result::<Self, _>(T::from_bytes(byte_stream, None))?;
        let c0r1 = conversion_result::<Self, _>(T::from_bytes(byte_stream, None))?;
        let c0r2 = conversion_result::<Self, _>(T::from_bytes(byte_stream, None))?;

        let c1r0 = conversion_result::<Self, _>(T::from_bytes(byte_stream, None))?;
        let c1r1 = conversion_result::<Self, _>(T::from_bytes(byte_stream, None))?;
        let c1r2 = conversion_result::<Self, _>(T::from_bytes(byte_stream, None))?;

        let c2r0 = conversion_result::<Self, _>(T::from_bytes(byte_stream, None))?;
        let c2r1 = conversion_result::<Self, _>(T::from_bytes(byte_stream, None))?;
        let c2r2 = conversion_result::<Self, _>(T::from_bytes(byte_stream, None))?;

        Ok(Matrix3::new(c0r0, c0r1, c0r2, c1r0, c1r1, c1r2, c2r0, c2r1, c2r2))
    }
}

#[cfg(test)]
mod default_string {
    use crate::loaders::{ByteStream, FromBytes, ToBytes};

    #[test]
    fn serialization_test() {
        let test_value = String::from("test");
        let data = test_value.to_bytes(None).unwrap();
        assert_eq!(data, vec![116, 101, 115, 116, 0]);
    }

    #[test]
    fn deserialization_test() {
        let mut byte_stream = ByteStream::new(&[116, 101, 115, 116, 0]);
        let test_value = String::from_bytes(&mut byte_stream, None).unwrap();
        assert_eq!(test_value.as_str(), "test");
        assert!(byte_stream.is_empty());
    }
}

#[cfg(test)]
mod length_hint_string {
    use crate::loaders::{ByteStream, FromBytes, ToBytes};

    #[test]
    fn serialization_test() {
        let test_value = String::from("test");
        let data = test_value.to_bytes(Some(8)).unwrap();
        assert_eq!(data, vec![116, 101, 115, 116, 0, 0, 0, 0]);
    }

    #[test]
    fn deserialization_test() {
        let mut byte_stream = ByteStream::new(&[116, 101, 115, 116, 0, 0, 0, 0]);
        let test_value = String::from_bytes(&mut byte_stream, Some(8)).unwrap();
        assert_eq!(test_value.as_str(), "test");
        assert!(byte_stream.is_empty());
    }
}

#[cfg(test)]
mod const_length_hint_string {
    use derive_new::new;
    use procedural::*;

    use crate::loaders::{ByteStream, FromBytes, ToBytes};

    const LENGTH: usize = 8;

    #[derive(Named, ByteConvertable, new)]
    struct TestStruct {
        #[length_hint(LENGTH)]
        pub string: String,
    }

    #[test]
    fn serialization_test() {
        let test_value = TestStruct::new("test".to_string());
        let data = test_value.to_bytes(None).unwrap();
        assert_eq!(data, vec![116, 101, 115, 116, 0, 0, 0, 0]);
    }

    #[test]
    fn deserialization_test() {
        let mut byte_stream = ByteStream::new(&[116, 101, 115, 116, 0, 0, 0, 0]);
        let test_value = TestStruct::from_bytes(&mut byte_stream, None).unwrap();
        assert_eq!(test_value.string.as_str(), "test");
        assert!(byte_stream.is_empty());
    }
}

#[cfg(test)]
mod dynamic_length_hint_string {
    use derive_new::new;
    use procedural::*;

    use crate::loaders::{ByteStream, FromBytes, ToBytes};

    #[derive(Named, Debug, PartialEq, ByteConvertable, new)]
    struct TestStruct {
        pub length: u8,
        #[length_hint(self.length * 2)]
        pub string: String,
    }

    #[test]
    fn serialization_test() {
        let test_value = TestStruct::new(4, "test".to_string());
        let data = test_value.to_bytes(None).unwrap();
        assert_eq!(data, vec![4, 116, 101, 115, 116, 0, 0, 0, 0]);
    }

    #[test]
    fn deserialization_test() {
        let mut byte_stream = ByteStream::new(&[4, 116, 101, 115, 116, 0, 0, 0, 0]);
        let test_value = TestStruct::from_bytes(&mut byte_stream, None).unwrap();
        assert_eq!(test_value, TestStruct::new(4, "test".to_string()));
        assert!(byte_stream.is_empty());
    }
}

#[cfg(test)]
mod default_struct {
    use derive_new::new;
    use procedural::*;

    use crate::loaders::{ByteStream, FromBytes, ToBytes};

    #[derive(Named, Debug, PartialEq, ByteConvertable, new)]
    struct TestStruct {
        pub field1: u8,
        pub field2: u16,
        pub field3: i32,
    }

    #[test]
    fn serialization_test() {
        let test_value = TestStruct::new(16, 3000, -1);
        let data = test_value.to_bytes(None).unwrap();
        assert_eq!(data, vec![16, 184, 11, 255, 255, 255, 255]);
    }

    #[test]
    fn deserialization_test() {
        let mut byte_stream = ByteStream::new(&[16, 184, 11, 255, 255, 255, 255]);
        let test_value = TestStruct::from_bytes(&mut byte_stream, None).unwrap();
        assert_eq!(test_value, TestStruct::new(16, 3000, -1));
        assert!(byte_stream.is_empty());
    }
}

#[cfg(test)]
mod version_struct_smaller {
    use derive_new::new;
    use procedural::*;

    use crate::loaders::{ByteStream, FromBytes, MajorFirst, Version};

    #[derive(Named, FromBytes, new)]
    struct TestStruct {
        #[version]
        pub _version: Version<MajorFirst>,
        #[version_smaller(4, 1)]
        pub maybe_value: Option<u32>,
    }

    #[test]
    fn deserialize_smaller() {
        let mut byte_stream = ByteStream::new(&[4, 0, 16, 0, 0, 0]);
        let test_value = TestStruct::from_bytes(&mut byte_stream, None).unwrap();
        assert_eq!(test_value.maybe_value, Some(16));
        assert!(byte_stream.is_empty());
    }

    #[test]
    fn deserialize_equals() {
        let mut byte_stream = ByteStream::new(&[4, 1, 16, 0, 0, 0]);
        let test_value = TestStruct::from_bytes(&mut byte_stream, None).unwrap();
        assert_eq!(test_value.maybe_value, None);
    }

    #[test]
    fn deserialize_bigger() {
        let mut byte_stream = ByteStream::new(&[4, 6, 16, 0, 0, 0]);
        let test_value = TestStruct::from_bytes(&mut byte_stream, None).unwrap();
        assert_eq!(test_value.maybe_value, None);
    }
}

#[cfg(test)]
mod version_struct_equals_or_above {
    use derive_new::new;
    use procedural::*;

    use crate::loaders::{ByteStream, FromBytes, MajorFirst, Version};

    #[derive(Named, FromBytes, new)]
    struct TestStruct {
        #[version]
        pub _version: Version<MajorFirst>,
        #[version_equals_or_above(4, 1)]
        pub maybe_value: Option<u32>,
    }

    #[test]
    fn deserialize_smaller() {
        let mut byte_stream = ByteStream::new(&[4, 0, 16, 0, 0, 0]);
        let test_value = TestStruct::from_bytes(&mut byte_stream, None).unwrap();
        assert_eq!(test_value.maybe_value, None);
    }

    #[test]
    fn deserialize_equals() {
        let mut byte_stream = ByteStream::new(&[4, 1, 16, 0, 0, 0]);
        let test_value = TestStruct::from_bytes(&mut byte_stream, None).unwrap();
        assert_eq!(test_value.maybe_value, Some(16));
        assert!(byte_stream.is_empty());
    }

    #[test]
    fn deserialize_bigger() {
        let mut byte_stream = ByteStream::new(&[4, 2, 16, 0, 0, 0]);
        let test_value = TestStruct::from_bytes(&mut byte_stream, None).unwrap();
        assert_eq!(test_value.maybe_value, Some(16));
        assert!(byte_stream.is_empty());
    }
}

#[cfg(test)]
mod default_enum {
    use procedural::*;

    use crate::loaders::{ByteStream, FromBytes, ToBytes};

    #[derive(Named, ByteConvertable)]
    enum TestEnum {
        First,
        Second,
        Third,
    }

    #[test]
    fn serialization_test() {
        let test_value = TestEnum::Second;
        let data = test_value.to_bytes(None).unwrap();
        assert_eq!(data, vec![1]);
    }

    #[test]
    fn deserialization_test() {
        let mut byte_stream = ByteStream::new(&[1]);
        let test_value = TestEnum::from_bytes(&mut byte_stream, None).unwrap();
        assert!(matches!(test_value, TestEnum::Second));
        assert!(byte_stream.is_empty());
    }
}

#[cfg(test)]
mod numeric_value_enum {
    use procedural::*;

    use crate::loaders::{ByteStream, FromBytes, ToBytes};

    #[derive(Named, ByteConvertable)]
    enum TestEnum {
        #[numeric_value(2)]
        First,
        #[numeric_value(10)]
        Second,
        #[numeric_value(255)]
        Third,
    }

    #[test]
    fn serialization_test() {
        let test_value = TestEnum::Second;
        let data = test_value.to_bytes(None).unwrap();
        assert_eq!(data, vec![10]);
    }

    #[test]
    fn deserialization_test() {
        let mut byte_stream = ByteStream::new(&[10]);
        let test_value = TestEnum::from_bytes(&mut byte_stream, None).unwrap();
        assert!(matches!(test_value, TestEnum::Second));
        assert!(byte_stream.is_empty());
    }
}

#[cfg(test)]
mod numeric_type_enum {
    use procedural::*;

    use crate::loaders::{ByteStream, FromBytes, ToBytes};

    #[derive(Named, ByteConvertable)]
    #[numeric_type(u16)]
    enum TestEnum {
        First,
        Second,
        Third,
    }

    #[test]
    fn serialization_test() {
        let test_value = TestEnum::Second;
        let data = test_value.to_bytes(None).unwrap();
        assert_eq!(data, vec![1, 0]);
    }

    #[test]
    fn deserialization_test() {
        let mut byte_stream = ByteStream::new(&[1, 0]);
        let test_value = TestEnum::from_bytes(&mut byte_stream, None).unwrap();
        assert!(matches!(test_value, TestEnum::Second));
        assert!(byte_stream.is_empty());
    }
}
