use crate::{ByteStream, ConversionResult};

mod implement;

/// Trait to deserialize from a [`ByteStream`].
pub trait FromBytes {
    /// Takes bytes from a [`ByteStream`] and deserializes them into a type `T`.
    fn from_bytes<Meta>(byte_stream: &mut ByteStream<Meta>) -> ConversionResult<Self>
    where
        Self: Sized;
}

/// Extension trait for [`FromBytes`].
pub trait FromBytesExt: FromBytes {
    /// Takes a fixed number of bytes from the [`ByteStream`] and tries to
    /// deserialize them into a type `T`.
    fn from_n_bytes<Meta>(byte_stream: &mut ByteStream<Meta>, size: usize) -> ConversionResult<Self>
    where
        Self: Sized;
}

impl<T> FromBytesExt for T
where
    T: FromBytes,
{
    #[allow(clippy::uninit_assumed_init)]
    fn from_n_bytes<Meta>(byte_stream: &mut ByteStream<Meta>, size: usize) -> ConversionResult<Self>
    where
        Self: Sized,
    {
        let stack_frame = byte_stream.install_limit::<Self>(size)?;

        let value = T::from_bytes(byte_stream)?;

        byte_stream.uninstall_limit(stack_frame);

        Ok(value)
    }
}

#[cfg(test)]
mod from_n_bytes {
    use super::FromBytes;
    use crate::{ByteStream, FromBytesExt};

    struct Test;

    const TEST_BYTE_SIZE: usize = 4;

    impl FromBytes for Test {
        fn from_bytes<Meta>(byte_stream: &mut crate::ByteStream<Meta>) -> crate::ConversionResult<Self>
        where
            Self: Sized,
        {
            byte_stream.slice::<Self>(TEST_BYTE_SIZE).map(|_| Test)
        }
    }

    #[test]
    fn data_saturated() {
        let mut byte_stream = ByteStream::<()>::without_metadata(&[0u8; TEST_BYTE_SIZE]);
        let result = Test::from_n_bytes(&mut byte_stream, TEST_BYTE_SIZE);

        assert!(result.is_ok());
        assert!(byte_stream.is_empty());
    }

    #[test]
    fn data_left() {
        let mut byte_stream = ByteStream::<()>::without_metadata(&[0u8; TEST_BYTE_SIZE * 2]);
        let result = Test::from_n_bytes(&mut byte_stream, TEST_BYTE_SIZE);

        assert!(result.is_ok());
        assert_eq!(byte_stream.remaining_bytes().len(), TEST_BYTE_SIZE);
    }

    #[test]
    fn data_missing() {
        let mut byte_stream = ByteStream::<()>::without_metadata(&[0u8; TEST_BYTE_SIZE]);
        let result = Test::from_n_bytes(&mut byte_stream, TEST_BYTE_SIZE * 2);

        assert!(result.is_err());

        // NOTE: This assert is checking an implementation detail that might well change
        // in the future.
        assert_eq!(byte_stream.remaining_bytes().len(), TEST_BYTE_SIZE);
    }

    #[test]
    fn incorrect_size() {
        let mut byte_stream = ByteStream::<()>::without_metadata(&[0u8; TEST_BYTE_SIZE]);
        let result = Test::from_n_bytes(&mut byte_stream, TEST_BYTE_SIZE / 2);

        assert!(result.is_err());

        // NOTE: This assert is checking an implementation detail that might well change
        // in the future.
        assert_eq!(byte_stream.remaining_bytes().len(), TEST_BYTE_SIZE / 2);
    }
}
