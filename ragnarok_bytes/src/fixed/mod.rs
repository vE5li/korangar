mod implement;

/// Trait for getting the size of Ragnarok Online types.
pub trait FixedByteSize {
    /// Get the serialized size in bytes.
    fn size_in_bytes() -> usize;
}

/// Trait for collections holding elements that implement [`FixedByteSize`].
pub trait FixedByteSizeCollection {
    /// Get the serialized size of the inner type in bytes.
    fn size_in_bytes() -> usize;
}
