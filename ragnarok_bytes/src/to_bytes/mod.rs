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
