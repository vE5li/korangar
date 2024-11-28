use crate::{ByteReader, ConversionResult};

mod implement;

/// Trait to deserialize from a [`ByteReader`].
pub trait FromBytes {
    /// Takes bytes from a [`ByteReader`] and deserializes them into a type `T`.
    fn from_bytes<Meta>(byte_reader: &mut ByteReader<Meta>) -> ConversionResult<Self>
    where
        Self: Sized;
}

/// Extension trait for [`FromBytes`].
pub trait FromBytesExt: FromBytes {
    /// Takes a fixed number of bytes from the [`ByteReader`] and tries to
    /// deserialize them into a type `T`.
    fn from_n_bytes<Meta>(byte_reader: &mut ByteReader<Meta>, size: usize) -> ConversionResult<Self>
    where
        Self: Sized;
}

impl<T> FromBytesExt for T
where
    T: FromBytes,
{
    #[allow(clippy::uninit_assumed_init)]
    fn from_n_bytes<Meta>(byte_reader: &mut ByteReader<Meta>, size: usize) -> ConversionResult<Self>
    where
        Self: Sized,
    {
        let stack_frame = byte_reader.install_limit::<Self>(size)?;

        let value = T::from_bytes(byte_reader)?;

        byte_reader.uninstall_limit(stack_frame);

        Ok(value)
    }
}

#[cfg(test)]
mod from_n_bytes {
    use super::FromBytes;
    use crate::{ByteReader, FromBytesExt};

    struct Test;

    const TEST_BYTE_SIZE: usize = 4;

    impl FromBytes for Test {
        fn from_bytes<Meta>(byte_reader: &mut crate::ByteReader<Meta>) -> crate::ConversionResult<Self>
        where
            Self: Sized,
        {
            byte_reader.slice::<Self>(TEST_BYTE_SIZE).map(|_| Test)
        }
    }

    #[test]
    fn data_saturated() {
        let mut byte_reader = ByteReader::<()>::without_metadata(&[0u8; TEST_BYTE_SIZE]);
        let result = Test::from_n_bytes(&mut byte_reader, TEST_BYTE_SIZE);

        assert!(result.is_ok());
        assert!(byte_reader.is_empty());
    }

    #[test]
    fn data_left() {
        let mut byte_reader = ByteReader::<()>::without_metadata(&[0u8; TEST_BYTE_SIZE * 2]);
        let result = Test::from_n_bytes(&mut byte_reader, TEST_BYTE_SIZE);

        assert!(result.is_ok());
        assert_eq!(byte_reader.remaining_bytes().len(), TEST_BYTE_SIZE);
    }

    #[test]
    fn data_missing() {
        let mut byte_reader = ByteReader::<()>::without_metadata(&[0u8; TEST_BYTE_SIZE]);
        let result = Test::from_n_bytes(&mut byte_reader, TEST_BYTE_SIZE * 2);

        assert!(result.is_err());

        // NOTE: This assert is checking an implementation detail that might well change
        // in the future.
        assert_eq!(byte_reader.remaining_bytes().len(), TEST_BYTE_SIZE);
    }

    #[test]
    fn incorrect_size() {
        let mut byte_reader = ByteReader::<()>::without_metadata(&[0u8; TEST_BYTE_SIZE]);
        let result = Test::from_n_bytes(&mut byte_reader, TEST_BYTE_SIZE / 2);

        assert!(result.is_err());

        // NOTE: This assert is checking an implementation detail that might well change
        // in the future.
        assert_eq!(byte_reader.remaining_bytes().len(), TEST_BYTE_SIZE / 2);
    }
}
