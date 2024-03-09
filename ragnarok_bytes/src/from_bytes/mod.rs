use crate::{ByteStream, ConversionResult, ConversionResultExt};

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
        // HACK: This will *not* work in the long run. This breaks as soon as the
        // metadata is written or read inside T::from_bytes.

        let slice = byte_stream.slice::<Self>(size)?;
        let mut hacked: ByteStream<()> = ByteStream::without_metadata(slice);

        T::from_bytes(&mut hacked).trace::<Self>()
    }
}
