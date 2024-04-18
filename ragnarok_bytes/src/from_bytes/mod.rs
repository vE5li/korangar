use crate::{ByteStream, ConversionResult};

mod implement;

/// Trait to deserialize from a [`ByteStream`].
pub trait FromBytes {
    /// Takes bytes from a [`ByteStream`] and deserializes them into a type `T`.
    fn from_bytes<META>(byte_stream: &mut ByteStream<META>) -> ConversionResult<Self>
    where
        Self: Sized;
}

/// Extension trait for [`FromBytes`].
pub trait FromBytesExt: FromBytes {
    /// Takes a fixed number of bytes from the [`ByteStream`] and tries to
    /// deserialize them into a type `T`.
    fn from_n_bytes<META>(byte_stream: &mut ByteStream<META>, size: usize) -> ConversionResult<Self>
    where
        Self: Sized;
}

impl<T> FromBytesExt for T
where
    T: FromBytes,
{
    #[allow(clippy::uninit_assumed_init)]
    fn from_n_bytes<META>(byte_stream: &mut ByteStream<META>, size: usize) -> ConversionResult<Self>
    where
        Self: Sized,
    {
        let stack_frame = byte_stream.install_limit::<Self>(size)?;

        let value = T::from_bytes(byte_stream)?;

        byte_stream.uninstall_limit(stack_frame);

        Ok(value)
    }
}
