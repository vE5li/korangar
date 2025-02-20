use crate::{ByteWriter, ConversionError, ConversionErrorType, ConversionResult};

mod implement;

/// Trait to serialize into bytes.
pub trait ToBytes {
    /// Converts self into bytes and write these into the [`ByteWriter`].
    ///
    /// Returns the count of the written bytes.
    fn to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize>;
}

/// Extension trait for [`ToBytes`].
pub trait ToBytesExt: ToBytes {
    /// Converts self into bytes, pads it with zeros to match the
    /// size of `size` and then writes these into the [`ByteWriter`].
    ///
    /// Returns the count of the written bytes.
    fn to_n_bytes(&self, byte_writer: &mut ByteWriter, size: usize) -> ConversionResult<usize>
    where
        Self: Sized;
}

impl<T> ToBytesExt for T
where
    T: ToBytes,
{
    fn to_n_bytes(&self, byte_writer: &mut ByteWriter, size: usize) -> ConversionResult<usize>
    where
        Self: Sized,
    {
        let written = T::to_bytes(self, byte_writer)?;

        match size.checked_sub(written) {
            None => {
                return Err(ConversionError::from_error_type(ConversionErrorType::DataTooBig {
                    type_name: std::any::type_name::<T>(),
                }));
            }
            Some(add_count) => {
                byte_writer.extend(add_count, 0);
            }
        }

        Ok(size)
    }
}

#[cfg(test)]
mod to_n_bytes {
    use super::ToBytes;
    use crate::{ByteWriter, ToBytesExt};

    struct Test;

    const TEST_BYTE_SIZE: usize = 4;

    impl ToBytes for Test {
        fn to_bytes(&self, byte_writer: &mut ByteWriter) -> crate::ConversionResult<usize> {
            byte_writer.write_counted(|writer| {
                writer.extend(TEST_BYTE_SIZE, 9);
                Ok(())
            })
        }
    }

    #[test]
    fn data_saturated() {
        let mut byte_writer = ByteWriter::new();
        let result = Test.to_n_bytes(&mut byte_writer, TEST_BYTE_SIZE);
        let bytes = byte_writer.into_inner();

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TEST_BYTE_SIZE);
        assert_eq!(bytes, vec![9; TEST_BYTE_SIZE]);
    }

    #[test]
    fn data_smaller() {
        let mut byte_writer = ByteWriter::new();
        let result = Test.to_n_bytes(&mut byte_writer, TEST_BYTE_SIZE * 2);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TEST_BYTE_SIZE * 2);

        let bytes = byte_writer.into_inner();
        assert_eq!(&bytes[..TEST_BYTE_SIZE], vec![9; TEST_BYTE_SIZE]);
        assert_eq!(&bytes[TEST_BYTE_SIZE..], vec![0; TEST_BYTE_SIZE]);
    }

    #[test]
    fn data_bigger() {
        let mut byte_writer = ByteWriter::new();
        let result = Test.to_n_bytes(&mut byte_writer, TEST_BYTE_SIZE / 2);

        assert!(result.is_err());
    }
}
