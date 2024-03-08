#![feature(const_trait_impl)]

mod error;
// FIX: Not moved for now since it makes the compiler crash :)
// mod fixed;
mod from_bytes;
mod helper;
mod stream;
mod to_bytes;

pub use self::error::{ConversionError, ConversionErrorType};
// FIX: Not moved for now since it makes the compiler crash :)
// pub use self::fixed::{FixedByteSize, FixedByteSizeWrapper};
pub use self::from_bytes::{FromBytes, FromBytesExt};
pub use self::helper::{ConversionResult, ConversionResultExt};
pub use self::stream::ByteStream;
pub use self::to_bytes::{ToBytes, ToBytesExt};

// #[cfg(test)]
// mod default_string {
//     use super::{ByteStream, FromBytes, ToBytes};
//
//     #[test]
//     fn serialization_test() {
//         let test_value = String::from("test");
//         let data = test_value.to_bytes().unwrap();
//         assert_eq!(data, vec![116, 101, 115, 116, 0]);
//     }
//
//     #[test]
//     fn deserialization_test() {
//         let mut byte_stream = ByteStream::<()>::without_metadata(&[116, 101,
// 115, 116, 0]);         let test_value = String::from_bytes(&mut
// byte_stream).unwrap();         assert_eq!(test_value.as_str(), "test");
//         assert!(byte_stream.is_empty());
//     }
// }
//
// #[cfg(test)]
// mod length_hint_string {
//     use super::{ByteStream, FromBytesExt, ToBytesExt};
//
//     #[test]
//     fn serialization_test() {
//         let test_value = String::from("test");
//         let data = test_value.to_n_bytes(8).unwrap();
//         assert_eq!(data, vec![116, 101, 115, 116, 0, 0, 0, 0]);
//     }
//
//     #[test]
//     fn deserialization_test() {
//         let mut byte_stream = ByteStream::<()>::without_metadata(&[116, 101,
// 115, 116, 0, 0, 0, 0]);         let test_value = String::from_n_bytes(&mut
// byte_stream, 8).unwrap();         assert_eq!(test_value.as_str(), "test");
//         assert_eq!(byte_stream.get_offset(), 8);
//     }
// }
//
// #[cfg(test)]
// mod const_length_hint_string {
//     use derive_new::new;
//     use procedural::*;
//
//     use super::{ByteStream, FromBytes, ToBytes};
//
//     const LENGTH: usize = 8;
//
//     #[derive(Named, ByteConvertable, new)]
//     struct TestStruct {
//         #[length_hint(LENGTH)]
//         pub string: String,
//     }
//
//     #[test]
//     fn serialization_test() {
//         let test_value = TestStruct::new("test".to_string());
//         let data = test_value.to_bytes().unwrap();
//         assert_eq!(data, vec![116, 101, 115, 116, 0, 0, 0, 0]);
//     }
//
//     #[test]
//     fn deserialization_test() {
//         let mut byte_stream = ByteStream::<()>::without_metadata(&[116, 101,
// 115, 116, 0, 0, 0, 0]);         let test_value = TestStruct::from_bytes(&mut
// byte_stream).unwrap();         assert_eq!(test_value.string.as_str(),
// "test");         assert!(byte_stream.is_empty());
//     }
// }
//
// #[cfg(test)]
// mod dynamic_length_hint_string {
//     use derive_new::new;
//     use procedural::*;
//
//     use super::{ByteStream, FromBytes, ToBytes};
//
//     #[derive(Named, Debug, PartialEq, ByteConvertable, new)]
//     struct TestStruct {
//         pub length: u8,
//         #[length_hint(self.length * 2)]
//         pub string: String,
//     }
//
//     #[test]
//     fn serialization_test() {
//         let test_value = TestStruct::new(4, "test".to_string());
//         let data = test_value.to_bytes().unwrap();
//         assert_eq!(data, vec![4, 116, 101, 115, 116, 0, 0, 0, 0]);
//     }
//
//     #[test]
//     fn deserialization_test() {
//         let mut byte_stream = ByteStream::<()>::without_metadata(&[4, 116,
// 101, 115, 116, 0, 0, 0, 0]);         let test_value =
// TestStruct::from_bytes(&mut byte_stream).unwrap();         assert_eq!
// (test_value, TestStruct::new(4, "test".to_string()));         assert!
// (byte_stream.is_empty());     }
// }
//
// #[cfg(test)]
// mod default_struct {
//     use derive_new::new;
//     use procedural::*;
//
//     use super::{ByteStream, FromBytes, ToBytes};
//
//     #[derive(Named, Debug, PartialEq, ByteConvertable, new)]
//     struct TestStruct {
//         pub field1: u8,
//         pub field2: u16,
//         pub field3: i32,
//     }
//
//     #[test]
//     fn serialization_test() {
//         let test_value = TestStruct::new(16, 3000, -1);
//         let data = test_value.to_bytes().unwrap();
//         assert_eq!(data, vec![16, 184, 11, 255, 255, 255, 255]);
//     }
//
//     #[test]
//     fn deserialization_test() {
//         let mut byte_stream = ByteStream::<()>::without_metadata(&[16, 184,
// 11, 255, 255, 255, 255]);         let test_value =
// TestStruct::from_bytes(&mut byte_stream).unwrap();         assert_eq!
// (test_value, TestStruct::new(16, 3000, -1));         assert!(byte_stream.
// is_empty());     }
// }
//
// #[cfg(test)]
// mod version_struct_smaller {
//     use derive_new::new;
//     use procedural::*;
//
//     use super::{ByteStream, FromBytes, InternalVersion, MajorFirst, Version};
//
//     #[derive(Named, FromBytes, new)]
//     struct TestStruct {
//         #[version]
//         pub _version: Version<MajorFirst>,
//         #[version_smaller(4, 1)]
//         pub maybe_value: Option<u32>,
//     }
//
//     #[test]
//     fn deserialize_smaller() {
//         let mut byte_stream =
// ByteStream::<Option<InternalVersion>>::without_metadata(&[4, 0, 16, 0, 0,
// 0]);         let test_value = TestStruct::from_bytes(&mut
// byte_stream).unwrap();         assert_eq!(test_value.maybe_value, Some(16));
//         assert!(byte_stream.is_empty());
//     }
//
//     #[test]
//     fn deserialize_equals() {
//         let mut byte_stream =
// ByteStream::<Option<InternalVersion>>::without_metadata(&[4, 1, 16, 0, 0,
// 0]);         let test_value = TestStruct::from_bytes(&mut
// byte_stream).unwrap();         assert_eq!(test_value.maybe_value, None);
//     }
//
//     #[test]
//     fn deserialize_bigger() {
//         let mut byte_stream =
// ByteStream::<Option<InternalVersion>>::without_metadata(&[4, 6, 16, 0, 0,
// 0]);         let test_value = TestStruct::from_bytes(&mut
// byte_stream).unwrap();         assert_eq!(test_value.maybe_value, None);
//     }
// }
//
// #[cfg(test)]
// mod version_struct_equals_or_above {
//     use derive_new::new;
//     use procedural::*;
//
//     use super::{ByteStream, FromBytes, InternalVersion, MajorFirst, Version};
//
//     #[derive(Named, FromBytes, new)]
//     struct TestStruct {
//         #[version]
//         pub _version: Version<MajorFirst>,
//         #[version_equals_or_above(4, 1)]
//         pub maybe_value: Option<u32>,
//     }
//
//     #[test]
//     fn deserialize_smaller() {
//         let mut byte_stream =
// ByteStream::<Option<InternalVersion>>::without_metadata(&[4, 0, 16, 0, 0,
// 0]);         let test_value = TestStruct::from_bytes(&mut
// byte_stream).unwrap();         assert_eq!(test_value.maybe_value, None);
//     }
//
//     #[test]
//     fn deserialize_equals() {
//         let mut byte_stream =
// ByteStream::<Option<InternalVersion>>::without_metadata(&[4, 1, 16, 0, 0,
// 0]);         let test_value = TestStruct::from_bytes(&mut
// byte_stream).unwrap();         assert_eq!(test_value.maybe_value, Some(16));
//         assert_eq!(byte_stream.get_offset(), 6);
//     }
//
//     #[test]
//     fn deserialize_bigger() {
//         let mut byte_stream =
// ByteStream::<Option<InternalVersion>>::without_metadata(&[4, 2, 16, 0, 0,
// 0]);         let test_value = TestStruct::from_bytes(&mut
// byte_stream).unwrap();         assert_eq!(test_value.maybe_value, Some(16));
//         assert_eq!(byte_stream.get_offset(), 6);
//     }
// }
//
// #[cfg(test)]
// mod default_enum {
//     use procedural::*;
//
//     use super::{ByteStream, FromBytes, ToBytes};
//
//     #[derive(Named, ByteConvertable)]
//     enum TestEnum {
//         First,
//         Second,
//         Third,
//     }
//
//     #[test]
//     fn serialization_test() {
//         let test_value = TestEnum::Second;
//         let data = test_value.to_bytes().unwrap();
//         assert_eq!(data, vec![1]);
//     }
//
//     #[test]
//     fn deserialization_test() {
//         let mut byte_stream = ByteStream::<()>::without_metadata(&[1]);
//         let test_value = TestEnum::from_bytes(&mut byte_stream).unwrap();
//         assert!(matches!(test_value, TestEnum::Second));
//         assert_eq!(byte_stream.get_offset(), 1);
//     }
// }
//
// #[cfg(test)]
// mod numeric_value_enum {
//     use procedural::*;
//
//     use super::{ByteStream, FromBytes, ToBytes};
//
//     #[derive(Named, ByteConvertable)]
//     enum TestEnum {
//         #[numeric_value(2)]
//         First,
//         #[numeric_value(10)]
//         Second,
//         #[numeric_value(255)]
//         Third,
//     }
//
//     #[test]
//     fn serialization_test() {
//         let test_value = TestEnum::Second;
//         let data = test_value.to_bytes().unwrap();
//         assert_eq!(data, vec![10]);
//     }
//
//     #[test]
//     fn deserialization_test() {
//         let mut byte_stream = ByteStream::<()>::without_metadata(&[10]);
//         let test_value = TestEnum::from_bytes(&mut byte_stream).unwrap();
//         assert!(matches!(test_value, TestEnum::Second));
//         assert_eq!(byte_stream.get_offset(), 1);
//     }
// }
//
// #[cfg(test)]
// mod numeric_type_enum {
//     use procedural::*;
//
//     use super::{ByteStream, FromBytes, ToBytes};
//
//     #[derive(Named, ByteConvertable)]
//     #[numeric_type(u16)]
//     enum TestEnum {
//         First,
//         Second,
//         Third,
//     }
//
//     #[test]
//     fn serialization_test() {
//         let test_value = TestEnum::Second;
//         let data = test_value.to_bytes().unwrap();
//         assert_eq!(data, vec![1, 0]);
//     }
//
//     #[test]
//     fn deserialization_test() {
//         let mut byte_stream = ByteStream::<()>::without_metadata(&[1, 0]);
//         let test_value = TestEnum::from_bytes(&mut byte_stream).unwrap();
//         assert!(matches!(test_value, TestEnum::Second));
//         assert_eq!(byte_stream.get_offset(), 2);
//     }
// }
