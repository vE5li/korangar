use crate::{ConversionError, ConversionErrorType, ConversionResult};

mod implement;

/// Trait to serialize into bytes.
pub trait ToBytes {
    /// Converts self to a [`Vec`] of bytes.
    fn to_bytes(&self) -> ConversionResult<Vec<u8>>;
}

/// Extension trait for [`ToBytes`].
pub trait ToBytesExt: ToBytes {
    /// Converts self to a [`Vec`] of bytes and pads it with zeros to match the
    /// size of `size`.
    fn to_n_bytes(&self, size: usize) -> ConversionResult<Vec<u8>>
    where
        Self: Sized;
}

impl<T> ToBytesExt for T
where
    T: ToBytes,
{
    fn to_n_bytes(&self, size: usize) -> ConversionResult<Vec<u8>>
    where
        Self: Sized,
    {
        let mut data = T::to_bytes(self)?;

        if data.len() > size {
            return Err(ConversionError::from_error_type(ConversionErrorType::DataTooBig {
                type_name: std::any::type_name::<T>(),
            }));
        }

        data.resize(size, 0);
        Ok(data)
    }
}

#[cfg(test)]
mod to_n_bytes {
    use super::ToBytes;
    use crate::ToBytesExt;

    struct Test;

    const TEST_BYTE_SIZE: usize = 4;

    impl ToBytes for Test {
        fn to_bytes(&self) -> crate::ConversionResult<Vec<u8>> {
            Ok(vec![9; TEST_BYTE_SIZE])
        }
    }

    #[test]
    fn data_saturated() {
        let result = Test.to_n_bytes(TEST_BYTE_SIZE);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![9; TEST_BYTE_SIZE]);
    }

    #[test]
    fn data_smaller() {
        let result = Test.to_n_bytes(TEST_BYTE_SIZE * 2);

        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(&data[..TEST_BYTE_SIZE], vec![9; TEST_BYTE_SIZE]);
        assert_eq!(&data[TEST_BYTE_SIZE..], vec![0; TEST_BYTE_SIZE]);
    }

    #[test]
    fn data_bigger() {
        let result = Test.to_n_bytes(TEST_BYTE_SIZE / 2);

        assert!(result.is_err());
    }
}
