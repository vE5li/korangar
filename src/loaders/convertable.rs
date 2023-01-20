use cgmath::{Matrix3, Quaternion, Vector2, Vector3, Vector4};

use crate::loaders::ByteStream;

pub trait ByteConvertable {
    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self;

    fn to_bytes(&self, _length_hint: Option<usize>) -> Vec<u8> {
        panic!()
    }
}

impl ByteConvertable for u8 {
    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
        assert!(length_hint.is_none(), "u8 may not have a length hint");
        byte_stream.next()
    }

    fn to_bytes(&self, length_hint: Option<usize>) -> Vec<u8> {
        assert!(length_hint.is_none(), "u8 may not have a length hint");
        vec![*self]
    }
}

impl ByteConvertable for u16 {
    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
        assert!(length_hint.is_none(), "u16 may not have a length hint");
        Self::from_le_bytes([byte_stream.next(), byte_stream.next()])
    }

    fn to_bytes(&self, length_hint: Option<usize>) -> Vec<u8> {
        assert!(length_hint.is_none(), "u16 may not have a length hint");
        self.to_le_bytes().to_vec()
    }
}

impl ByteConvertable for u32 {
    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
        assert!(length_hint.is_none(), "u32 may not have a length hint");
        Self::from_le_bytes([byte_stream.next(), byte_stream.next(), byte_stream.next(), byte_stream.next()])
    }

    fn to_bytes(&self, length_hint: Option<usize>) -> Vec<u8> {
        assert!(length_hint.is_none(), "u32 may not have a length hint");
        self.to_le_bytes().to_vec()
    }
}

impl ByteConvertable for u64 {
    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
        assert!(length_hint.is_none(), "u64 may not have a length hint");
        Self::from_le_bytes([
            byte_stream.next(),
            byte_stream.next(),
            byte_stream.next(),
            byte_stream.next(),
            byte_stream.next(),
            byte_stream.next(),
            byte_stream.next(),
            byte_stream.next(),
        ])
    }

    fn to_bytes(&self, length_hint: Option<usize>) -> Vec<u8> {
        assert!(length_hint.is_none(), "u64 may not have a length hint");
        self.to_le_bytes().to_vec()
    }
}

impl ByteConvertable for i8 {
    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
        assert!(length_hint.is_none(), "i8 may not have a length hint");
        byte_stream.next() as i8
    }

    fn to_bytes(&self, length_hint: Option<usize>) -> Vec<u8> {
        assert!(length_hint.is_none(), "i8 may not have a length hint");
        vec![*self as u8]
    }
}

impl ByteConvertable for i16 {
    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
        assert!(length_hint.is_none(), "i16 may not have a length hint");
        Self::from_le_bytes([byte_stream.next(), byte_stream.next()])
    }

    fn to_bytes(&self, length_hint: Option<usize>) -> Vec<u8> {
        assert!(length_hint.is_none(), "i16 may not have a length hint");
        self.to_le_bytes().to_vec()
    }
}

impl ByteConvertable for i32 {
    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
        assert!(length_hint.is_none(), "i32 may not have a length hint");
        Self::from_le_bytes([byte_stream.next(), byte_stream.next(), byte_stream.next(), byte_stream.next()])
    }

    fn to_bytes(&self, length_hint: Option<usize>) -> Vec<u8> {
        assert!(length_hint.is_none(), "i32 may not have a length hint");
        self.to_le_bytes().to_vec()
    }
}

impl ByteConvertable for i64 {
    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
        assert!(length_hint.is_none(), "i64 may not have a length hint");
        Self::from_le_bytes([
            byte_stream.next(),
            byte_stream.next(),
            byte_stream.next(),
            byte_stream.next(),
            byte_stream.next(),
            byte_stream.next(),
            byte_stream.next(),
            byte_stream.next(),
        ])
    }

    fn to_bytes(&self, length_hint: Option<usize>) -> Vec<u8> {
        assert!(length_hint.is_none(), "i64 may not have a length hint");
        self.to_le_bytes().to_vec()
    }
}

impl ByteConvertable for f32 {
    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
        assert!(length_hint.is_none(), "f32 may not have a length hint");
        Self::from_le_bytes([byte_stream.next(), byte_stream.next(), byte_stream.next(), byte_stream.next()])
    }

    fn to_bytes(&self, length_hint: Option<usize>) -> Vec<u8> {
        assert!(length_hint.is_none(), "f32 may not have a length hint");
        self.to_ne_bytes().to_vec()
    }
}

impl<T: ByteConvertable, const SIZE: usize> ByteConvertable for [T; SIZE] {
    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
        use std::mem::MaybeUninit;

        assert!(length_hint.is_none(), "array may not have a length hint");

        let mut data: [MaybeUninit<T>; SIZE] = unsafe { MaybeUninit::uninit().assume_init() };

        for element in &mut data[..] {
            *element = MaybeUninit::new(T::from_bytes(byte_stream, None));
        }

        // rust wont let us do this currently
        //unsafe { mem::transmute::<_, [T; SIZE]>(data) }

        // workaround from: https://github.com/rust-lang/rust/issues/61956
        let ptr = &mut data as *mut _ as *mut [T; SIZE];
        let result = unsafe { ptr.read() };
        core::mem::forget(data);
        result
    }

    fn to_bytes(&self, length_hint: Option<usize>) -> Vec<u8> {
        assert!(length_hint.is_none(), "array may not have a length hint");

        self.iter().fold(Vec::new(), |mut bytes, value| {
            bytes.extend(value.to_bytes(None));
            bytes
        })
    }
}

impl ByteConvertable for String {
    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
        let mut value = String::new();
        let mut offset = 0;

        loop {
            offset += 1;

            match byte_stream.next() {
                0 => break,
                byte => value.push(byte as char),
            }
        }

        if let Some(length) = length_hint {
            byte_stream.skip(length - offset);
        }

        value
    }

    fn to_bytes(&self, length_hint: Option<usize>) -> Vec<u8> {
        use std::iter;

        match length_hint {
            Some(length) => {
                assert!(self.len() <= length, "string is to long for the byte stream");
                let padding = (0..length - self.len()).map(|_| 0);
                self.bytes().chain(padding).collect()
            }
            None => self.bytes().chain(iter::once(0)).collect(),
        }
    }
}

impl<T: ByteConvertable> ByteConvertable for Vec<T> {
    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
        let length = length_hint.expect("vector requires a size hint");
        let data = byte_stream.slice(length);
        let mut byte_stream = ByteStream::new(&data);
        let mut vector = Vec::new();

        while !byte_stream.is_empty() {
            vector.push(T::from_bytes(&mut byte_stream, None));
        }

        vector
    }
}

impl<T: ByteConvertable> ByteConvertable for Vector2<T> {
    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
        assert!(length_hint.is_none(), "vector2 may not have a length hint");

        let first = T::from_bytes(byte_stream, None);
        let second = T::from_bytes(byte_stream, None);

        Vector2::new(first, second)
    }
}

impl<T: ByteConvertable> ByteConvertable for Vector3<T> {
    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
        assert!(length_hint.is_none(), "vector3 may not have a length hint");

        let first = T::from_bytes(byte_stream, None);
        let second = T::from_bytes(byte_stream, None);
        let third = T::from_bytes(byte_stream, None);

        Vector3::new(first, second, third)
    }
}

impl<T: ByteConvertable> ByteConvertable for Vector4<T> {
    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
        assert!(length_hint.is_none(), "vector4 may not have a length hint");

        let first = T::from_bytes(byte_stream, None);
        let second = T::from_bytes(byte_stream, None);
        let third = T::from_bytes(byte_stream, None);
        let fourth = T::from_bytes(byte_stream, None);

        Vector4::new(first, second, third, fourth)
    }
}

impl<T: ByteConvertable> ByteConvertable for Quaternion<T> {
    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
        assert!(length_hint.is_none(), "quaternion may not have a length hint");

        let first = T::from_bytes(byte_stream, None);
        let second = T::from_bytes(byte_stream, None);
        let third = T::from_bytes(byte_stream, None);
        let fourth = T::from_bytes(byte_stream, None);

        Quaternion::new(fourth, first, second, third)
    }
}

impl<T: ByteConvertable> ByteConvertable for Matrix3<T> {
    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
        assert!(length_hint.is_none(), "matrix may not have a length hint");

        let c0r0 = T::from_bytes(byte_stream, None);
        let c0r1 = T::from_bytes(byte_stream, None);
        let c0r2 = T::from_bytes(byte_stream, None);

        let c1r0 = T::from_bytes(byte_stream, None);
        let c1r1 = T::from_bytes(byte_stream, None);
        let c1r2 = T::from_bytes(byte_stream, None);

        let c2r0 = T::from_bytes(byte_stream, None);
        let c2r1 = T::from_bytes(byte_stream, None);
        let c2r2 = T::from_bytes(byte_stream, None);

        Matrix3::new(c0r0, c0r1, c0r2, c1r0, c1r1, c1r2, c2r0, c2r1, c2r2)
    }
}

#[cfg(test)]
mod default_string {

    use crate::loaders::{ByteConvertable, ByteStream};

    #[test]
    fn serialization_test() {
        let test_value = String::from("test");
        let data = test_value.to_bytes(None);
        assert_eq!(data, vec![116, 101, 115, 116, 0]);
    }

    #[test]
    fn deserialization_test() {
        let mut byte_stream = ByteStream::new(&[116, 101, 115, 116, 0]);
        let test_value = String::from_bytes(&mut byte_stream, None);
        assert_eq!(test_value.as_str(), "test");
        assert!(byte_stream.is_empty());
    }
}

#[cfg(test)]
mod length_hint_string {

    use crate::loaders::{ByteConvertable, ByteStream};

    #[test]
    fn serialization_test() {
        let test_value = String::from("test");
        let data = test_value.to_bytes(Some(8));
        assert_eq!(data, vec![116, 101, 115, 116, 0, 0, 0, 0]);
    }

    #[test]
    fn deserialization_test() {
        let mut byte_stream = ByteStream::new(&[116, 101, 115, 116, 0, 0, 0, 0]);
        let test_value = String::from_bytes(&mut byte_stream, Some(8));
        assert_eq!(test_value.as_str(), "test");
        assert!(byte_stream.is_empty());
    }
}

#[cfg(test)]
mod const_length_hint_string {

    use derive_new::new;
    use procedural::*;

    use crate::loaders::{ByteConvertable, ByteStream};

    const LENGTH: usize = 8;

    #[derive(ByteConvertable, new)]
    struct TestStruct {
        #[length_hint(LENGTH)]
        pub string: String,
    }

    #[test]
    fn serialization_test() {
        let test_value = TestStruct::new("test".to_string());
        let data = test_value.to_bytes(None);
        assert_eq!(data, vec![116, 101, 115, 116, 0, 0, 0, 0]);
    }

    #[test]
    fn deserialization_test() {
        let mut byte_stream = ByteStream::new(&[116, 101, 115, 116, 0, 0, 0, 0]);
        let test_value = TestStruct::from_bytes(&mut byte_stream, None);
        assert_eq!(test_value.string.as_str(), "test");
        assert!(byte_stream.is_empty());
    }
}

#[cfg(test)]
mod dynamic_length_hint_string {

    use derive_new::new;
    use procedural::*;

    use crate::loaders::{ByteConvertable, ByteStream};

    #[derive(Debug, PartialEq, ByteConvertable, new)]
    struct TestStruct {
        pub length: u8,
        #[length_hint(self.length * 2)]
        pub string: String,
    }

    #[test]
    fn serialization_test() {
        let test_value = TestStruct::new(4, "test".to_string());
        let data = test_value.to_bytes(None);
        assert_eq!(data, vec![4, 116, 101, 115, 116, 0, 0, 0, 0]);
    }

    #[test]
    fn deserialization_test() {
        let mut byte_stream = ByteStream::new(&[4, 116, 101, 115, 116, 0, 0, 0, 0]);
        let test_value = TestStruct::from_bytes(&mut byte_stream, None);
        assert_eq!(test_value, TestStruct::new(4, "test".to_string()));
        assert!(byte_stream.is_empty());
    }
}

#[cfg(test)]
mod default_struct {

    use derive_new::new;
    use procedural::*;

    use crate::loaders::{ByteConvertable, ByteStream};

    #[derive(Debug, PartialEq, ByteConvertable, new)]
    struct TestStruct {
        pub field1: u8,
        pub field2: u16,
        pub field3: i32,
    }

    #[test]
    fn serialization_test() {
        let test_value = TestStruct::new(16, 3000, -1);
        let data = test_value.to_bytes(None);
        assert_eq!(data, vec![16, 184, 11, 255, 255, 255, 255]);
    }

    #[test]
    fn deserialization_test() {
        let mut byte_stream = ByteStream::new(&[16, 184, 11, 255, 255, 255, 255]);
        let test_value = TestStruct::from_bytes(&mut byte_stream, None);
        assert_eq!(test_value, TestStruct::new(16, 3000, -1));
        assert!(byte_stream.is_empty());
    }
}

// TODO: reenable once the versioning is fixed
/*
#[cfg(test)]
mod version_struct_smaller {

    use procedural::*;
    use derive_new::new;
    use crate::loaders::{ ByteStream, ByteConvertable };
    use crate::loaders::Version;

    #[derive(ByteConvertable, new)]
    struct TestStruct {
        #[version]
        pub version: Version,
        #[version_smaller(4, 1)]
        pub maybe_value: Option<u32>,
    }

    #[test]
    fn deserialize_smaller() {
        let mut byte_stream = ByteStream::new(&[4, 0, 16, 0, 0, 0]);
        let test_value = TestStruct::from_bytes(&mut byte_stream, None);
        assert_eq!(test_value.maybe_value, Some(16));
        assert!(byte_stream.is_empty());
    }

    #[test]
    fn deserialize_equals() {
        let mut byte_stream = ByteStream::new(&[4, 1, 16, 0, 0, 0]);
        let test_value = TestStruct::from_bytes(&mut byte_stream, None);
        assert_eq!(test_value.maybe_value, None);
    }

    #[test]
    fn deserialize_bigger() {
        let mut byte_stream = ByteStream::new(&[4, 6, 16, 0, 0, 0]);
        let test_value = TestStruct::from_bytes(&mut byte_stream, None);
        assert_eq!(test_value.maybe_value, None);
    }
}

#[cfg(test)]
mod version_struct_bigger {

    use procedural::*;
    use derive_new::new;
    use crate::loaders::{ ByteStream, ByteConvertable };
    use crate::loaders::Version;

    #[derive(ByteConvertable, new)]
    struct TestStruct {
        #[version]
        pub version: Version,
        #[version_equals_or_above(4, 1)]
        pub maybe_value: Option<u32>,
    }

    #[test]
    fn deserialize_smaller() {
        let mut byte_stream = ByteStream::new(&[4, 0, 16, 0, 0, 0]);
        let test_value = TestStruct::from_bytes(&mut byte_stream, None);
        assert_eq!(test_value.maybe_value, None);
    }

    #[test]
    fn deserialize_equals() {
        let mut byte_stream = ByteStream::new(&[4, 1, 16, 0, 0, 0]);
        let test_value = TestStruct::from_bytes(&mut byte_stream, None);
        assert_eq!(test_value.maybe_value, Some(16));
        assert!(byte_stream.is_empty());
    }

    #[test]
    fn deserialize_bigger() {
        let mut byte_stream = ByteStream::new(&[4, 2, 16, 0, 0, 0]);
        let test_value = TestStruct::from_bytes(&mut byte_stream, None);
        assert_eq!(test_value.maybe_value, Some(16));
        assert!(byte_stream.is_empty());
    }
}
*/

#[cfg(test)]
mod default_enum {

    use procedural::*;

    use crate::loaders::{ByteConvertable, ByteStream};

    #[derive(ByteConvertable)]
    enum TestEnum {
        First,
        Second,
        Third,
    }

    #[test]
    fn serialization_test() {
        let test_value = TestEnum::Second;
        let data = test_value.to_bytes(None);
        assert_eq!(data, vec![1]);
    }

    #[test]
    fn deserialization_test() {
        let mut byte_stream = ByteStream::new(&[1]);
        let test_value = TestEnum::from_bytes(&mut byte_stream, None);
        assert!(matches!(test_value, TestEnum::Second));
        assert!(byte_stream.is_empty());
    }
}

#[cfg(test)]
mod numeric_value_enum {

    use procedural::*;

    use crate::loaders::{ByteConvertable, ByteStream};

    #[derive(ByteConvertable)]
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
        let data = test_value.to_bytes(None);
        assert_eq!(data, vec![10]);
    }

    #[test]
    fn deserialization_test() {
        let mut byte_stream = ByteStream::new(&[10]);
        let test_value = TestEnum::from_bytes(&mut byte_stream, None);
        assert!(matches!(test_value, TestEnum::Second));
        assert!(byte_stream.is_empty());
    }
}

#[cfg(test)]
mod numeric_type_enum {

    use procedural::*;

    use crate::loaders::{ByteConvertable, ByteStream};

    #[derive(ByteConvertable)]
    #[numeric_type(u16)]
    enum TestEnum {
        First,
        Second,
        Third,
    }

    #[test]
    fn serialization_test() {
        let test_value = TestEnum::Second;
        let data = test_value.to_bytes(None);
        assert_eq!(data, vec![1, 0]);
    }

    #[test]
    fn deserialization_test() {
        let mut byte_stream = ByteStream::new(&[1, 0]);
        let test_value = TestEnum::from_bytes(&mut byte_stream, None);
        assert!(matches!(test_value, TestEnum::Second));
        assert!(byte_stream.is_empty());
    }
}
