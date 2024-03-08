mod implement;

#[const_trait]
pub trait FixedByteSize {
    fn size_in_bytes() -> usize;
}

#[const_trait]
pub trait FixedByteSizeWrapper {
    fn size_in_bytes() -> usize;
}
