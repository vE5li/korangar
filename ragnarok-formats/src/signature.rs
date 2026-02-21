use std::fmt::Display;

use ragnarok_bytes::{ByteReader, ByteWriter, ConversionError, ConversionResult, FixedByteSize, FromBytes, ToBytes};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Signature<const MAGIC: &'static [u8]>;

impl<const MAGIC: &'static [u8]> FixedByteSize for Signature<MAGIC> {
    fn size_in_bytes() -> usize {
        MAGIC.len()
    }
}

impl<const MAGIC: &'static [u8]> FromBytes for Signature<MAGIC> {
    fn from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self>
    where
        Self: Sized,
    {
        let bytes = byte_reader.slice::<Self>(MAGIC.len())?;
        match bytes == MAGIC {
            true => Ok(Self),
            false => Err(ConversionError::from_message("invalid magic number")),
        }
    }
}

impl<const MAGIC: &'static [u8]> ToBytes for Signature<MAGIC> {
    fn to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        byte_writer.extend_from_slice(MAGIC);
        Ok(MAGIC.len())
    }
}

impl<const MAGIC: &'static [u8]> Display for Signature<MAGIC> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = std::str::from_utf8(MAGIC).expect("signature has to be UTF-8");
        write!(f, "{}", string)
    }
}
