use crate::types::ByteStream;
use crate::types::maths::*;

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

        let mut value = 0;

        value |= byte_stream.next() as u16;
        value |= (byte_stream.next() as u16) << 8;

        value
    }

    fn to_bytes(&self, length_hint: Option<usize>) -> Vec<u8> {
        assert!(length_hint.is_none(), "u16 may not have a length hint");
        vec![*self as u8, (*self >> 8) as u8]
    }
}

impl ByteConvertable for u32 {

    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
        assert!(length_hint.is_none(), "u32 may not have a length hint");

        let mut value = 0;

        value |= byte_stream.next() as u32;
        value |= (byte_stream.next() as u32) << 8;
        value |= (byte_stream.next() as u32) << 16;
        value |= (byte_stream.next() as u32) << 24;

        value
    }

    fn to_bytes(&self, length_hint: Option<usize>) -> Vec<u8> {
        assert!(length_hint.is_none(), "u32 may not have a length hint");
        vec![*self as u8, (*self >> 8) as u8, (*self >> 16) as u8, (*self >> 24) as u8]
    }
}

impl ByteConvertable for u64 {

    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
        assert!(length_hint.is_none(), "u64 may not have a length hint");

        let mut value = 0;

        value |= byte_stream.next() as u64;
        value |= (byte_stream.next() as u64) << 8;
        value |= (byte_stream.next() as u64) << 16;
        value |= (byte_stream.next() as u64) << 24;
        value |= (byte_stream.next() as u64) << 32;
        value |= (byte_stream.next() as u64) << 40;
        value |= (byte_stream.next() as u64) << 48;
        value |= (byte_stream.next() as u64) << 56;

        value
    }

    fn to_bytes(&self, length_hint: Option<usize>) -> Vec<u8> {
        assert!(length_hint.is_none(), "u64 may not have a length hint");
        vec![*self as u8, (*self >> 8) as u8, (*self >> 16) as u8, (*self >> 24) as u8, (*self >> 32) as u8, (*self >> 40) as u8, (*self >> 48) as u8, (*self >> 56) as u8]
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

        let mut value = 0;

        value |= byte_stream.next() as i16;
        value |= (byte_stream.next() as i16) << 8;

        value
    }

    fn to_bytes(&self, length_hint: Option<usize>) -> Vec<u8> {
        assert!(length_hint.is_none(), "i16 may not have a length hint");
        vec![*self as u8, (*self >> 8) as u8]
    }
}

impl ByteConvertable for i32 {

    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
        assert!(length_hint.is_none(), "i32 may not have a length hint");

        let mut value = 0;

        value |= byte_stream.next() as i32;
        value |= (byte_stream.next() as i32) << 8;
        value |= (byte_stream.next() as i32) << 16;
        value |= (byte_stream.next() as i32) << 24;

        value
    }

    fn to_bytes(&self, length_hint: Option<usize>) -> Vec<u8> {
        assert!(length_hint.is_none(), "i32 may not have a length hint");
        vec![*self as u8, (*self >> 8) as u8, (*self >> 16) as u8, (*self >> 24) as u8]
    }
}

impl ByteConvertable for i64 {

    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
        assert!(length_hint.is_none(), "i64 may not have a length hint");

        let mut value = 0;

        value |= byte_stream.next() as i64;
        value |= (byte_stream.next() as i64) << 8;
        value |= (byte_stream.next() as i64) << 16;
        value |= (byte_stream.next() as i64) << 24;
        value |= (byte_stream.next() as i64) << 32;
        value |= (byte_stream.next() as i64) << 40;
        value |= (byte_stream.next() as i64) << 48;
        value |= (byte_stream.next() as i64) << 56;

        value
    }

    fn to_bytes(&self, length_hint: Option<usize>) -> Vec<u8> {
        assert!(length_hint.is_none(), "i64 may not have a length hint");
        vec![*self as u8, (*self >> 8) as u8, (*self >> 16) as u8, (*self >> 24) as u8, (*self >> 32) as u8, (*self >> 40) as u8, (*self >> 48) as u8, (*self >> 56) as u8]
    }
}

impl ByteConvertable for f32 {

    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
        assert!(length_hint.is_none(), "i32 may not have a length hint");

        let first = byte_stream.next();
        let second = byte_stream.next();
        let third = byte_stream.next();
        let fourth = byte_stream.next();

        f32::from_le_bytes([first, second, third, fourth])
    }

    fn to_bytes(&self, length_hint: Option<usize>) -> Vec<u8> {
        assert!(length_hint.is_none(), "i32 may not have a length hint");
        self.to_ne_bytes().to_vec()
    }
}

impl<T: Copy + Default + ByteConvertable, const SIZE: usize> ByteConvertable for [T; SIZE] {

    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
        assert!(length_hint.is_none(), "array may not have a length hint");

        let mut value = [T::default(); SIZE];

        for index in 0..SIZE {
            value[index] = T::from_bytes(byte_stream, None);
        }

        value
    }

    fn to_bytes(&self, length_hint: Option<usize>) -> Vec<u8> {
        assert!(length_hint.is_none(), "array may not have a length hint");

        self
            .iter()
            .fold(Vec::new(), |mut bytes, value| {
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
                byte => value.push(byte as char)
            }
        }

        if let Some(length) = length_hint {
            byte_stream.skip(length - offset); 
            // maybe error if no zero byte was found
        }

        value
    }

    fn to_bytes(&self, length_hint: Option<usize>) -> Vec<u8> {
        use std::iter;

        match length_hint {

            Some(length) => {
                assert!(self.len() <= length, "string is to long for the byte stream");
                let padding = (0..length - self.len()).into_iter().map(|_| 0);
                self.bytes().chain(padding).collect()
            },

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

        let first = ByteConvertable::from_bytes(byte_stream, None);
        let second = ByteConvertable::from_bytes(byte_stream, None);

        Vector2::new(first, second)
    }
}

impl<T: ByteConvertable> ByteConvertable for Vector3<T> {

    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
        assert!(length_hint.is_none(), "vector3 may not have a length hint");

        let first = ByteConvertable::from_bytes(byte_stream, None);
        let second = ByteConvertable::from_bytes(byte_stream, None);
        let third = ByteConvertable::from_bytes(byte_stream, None);

        Vector3::new(first, second, third)
    }
}

impl<T: ByteConvertable> ByteConvertable for Vector4<T> {

    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
        assert!(length_hint.is_none(), "vector4 may not have a length hint");

        let first = ByteConvertable::from_bytes(byte_stream, None);
        let second = ByteConvertable::from_bytes(byte_stream, None);
        let third = ByteConvertable::from_bytes(byte_stream, None);
        let fourth = ByteConvertable::from_bytes(byte_stream, None);

        Vector4::new(first, second, third, fourth)
    }
}

impl<T: ByteConvertable> ByteConvertable for Matrix3<T> {

    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
        assert!(length_hint.is_none(), "matrix may not have a length hint");

        let c0r0 = ByteConvertable::from_bytes(byte_stream, None);
        let c0r1 = ByteConvertable::from_bytes(byte_stream, None);
        let c0r2 = ByteConvertable::from_bytes(byte_stream, None);

        let c1r0 = ByteConvertable::from_bytes(byte_stream, None);
        let c1r1 = ByteConvertable::from_bytes(byte_stream, None);
        let c1r2 = ByteConvertable::from_bytes(byte_stream, None);

        let c2r0 = ByteConvertable::from_bytes(byte_stream, None);
        let c2r1 = ByteConvertable::from_bytes(byte_stream, None);
        let c2r2 = ByteConvertable::from_bytes(byte_stream, None);

        Matrix3::new(c0r0, c0r1, c0r2, c1r0, c1r1, c1r2, c2r0, c2r1, c2r2)
    }
}


#[cfg(test)]
mod default_string {

    use crate::types::ByteStream;
    use crate::traits::ByteConvertable;

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

    use crate::types::ByteStream;
    use crate::traits::ByteConvertable;

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
    use crate::types::ByteStream;
    use crate::traits::ByteConvertable;

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
    use crate::types::ByteStream;
    use crate::traits::ByteConvertable;

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
    use crate::types::ByteStream;
    use crate::traits::ByteConvertable;

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

#[cfg(test)]
mod version_struct_smaller {

    use derive_new::new;
    use crate::types::ByteStream;
    use crate::traits::ByteConvertable;
    use crate::types::Version;

    #[derive(ByteConvertable, new)]
    struct TestStruct {
        #[version]
        pub version: Version,
        #[version_smaller(1, 1)]
        pub maybe_value: Option<u32>,
    }

    #[test]
    fn deserialize_smaller() {
        let mut byte_stream = ByteStream::new(&[1, 0, 16, 0, 0, 0]);
        let test_value = TestStruct::from_bytes(&mut byte_stream, None);
        assert_eq!(test_value.maybe_value, Some(16));
        assert!(byte_stream.is_empty());
    }

    #[test]
    fn deserialize_equals() {
        let mut byte_stream = ByteStream::new(&[1, 1, 16, 0, 0, 0]);
        let test_value = TestStruct::from_bytes(&mut byte_stream, None);
        assert_eq!(test_value.maybe_value, None);
    }

    #[test]
    fn deserialize_bigger() {
        let mut byte_stream = ByteStream::new(&[1, 6, 16, 0, 0, 0]);
        let test_value = TestStruct::from_bytes(&mut byte_stream, None);
        assert_eq!(test_value.maybe_value, None);
    }
}

#[cfg(test)]
mod version_struct_bigger {

    use derive_new::new;
    use crate::types::ByteStream;
    use crate::traits::ByteConvertable;
    use crate::types::Version;

    #[derive(ByteConvertable, new)]
    struct TestStruct {
        #[version]
        pub version: Version,
        #[version_equals_or_above(1, 1)]
        pub maybe_value: Option<u32>,
    }

    #[test]
    fn deserialize_smaller() {
        let mut byte_stream = ByteStream::new(&[1, 0, 16, 0, 0, 0]);
        let test_value = TestStruct::from_bytes(&mut byte_stream, None);
        assert_eq!(test_value.maybe_value, None);
    }

    #[test]
    fn deserialize_equals() {
        let mut byte_stream = ByteStream::new(&[1, 1, 16, 0, 0, 0]);
        let test_value = TestStruct::from_bytes(&mut byte_stream, None);
        assert_eq!(test_value.maybe_value, Some(16));
        assert!(byte_stream.is_empty());
    }

    #[test]
    fn deserialize_bigger() {
        let mut byte_stream = ByteStream::new(&[1, 6, 16, 0, 0, 0]);
        let test_value = TestStruct::from_bytes(&mut byte_stream, None);
        assert_eq!(test_value.maybe_value, Some(16));
        assert!(byte_stream.is_empty());
    }
}

#[cfg(test)]
mod default_enum {

    use crate::types::ByteStream;
    use crate::traits::ByteConvertable;

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
mod variant_value_enum {

    use crate::types::ByteStream;
    use crate::traits::ByteConvertable;

    #[derive(ByteConvertable)]
    enum TestEnum {
        #[variant_value(2)]
        First,
        #[variant_value(10)]
        Second,
        #[variant_value(255)]
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
mod base_type_enum {

    use crate::types::ByteStream;
    use crate::traits::ByteConvertable;

    #[derive(ByteConvertable)]
    #[base_type(u16)]
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
