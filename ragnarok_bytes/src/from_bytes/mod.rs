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
        use std::mem::MaybeUninit;

        // Move the metadata to a temporary memory slot.
        //
        // SAFETY: Obviously this not safe and will be removed in the future.
        let mut swap_metadata = unsafe { MaybeUninit::uninit().assume_init() };
        std::mem::swap(byte_stream.get_metadata_mut::<Self, META>()?, &mut swap_metadata);

        let (result, mut metadata) = {
            let data = byte_stream.slice::<T>(size)?;
            let mut byte_stream = ByteStream::<META>::with_metadata(data, swap_metadata);

            let result = T::from_bytes(&mut byte_stream);
            let metadata = byte_stream.into_metadata();

            (result, metadata)
        };

        // Move the metadata back to the original byte stream and forget the temporary
        // memory slot.
        std::mem::swap(byte_stream.get_metadata_mut::<Self, META>().unwrap(), &mut metadata);
        std::mem::forget(metadata);

        result
    }
}
