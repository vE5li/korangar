use ragnarok_bytes::{ByteStream, ConversionError, ConversionResult, FixedByteSize, FromBytes, ToBytes};

#[derive(Debug, Clone, Default)]
pub struct Signature<const MAGIC: &'static [u8]>;

impl<const MAGIC: &'static [u8]> FixedByteSize for Signature<MAGIC> {
    fn size_in_bytes() -> usize {
        MAGIC.len()
    }
}

impl<const MAGIC: &'static [u8]> FromBytes for Signature<MAGIC> {
    fn from_bytes<Meta>(byte_stream: &mut ByteStream<Meta>) -> ConversionResult<Self>
    where
        Self: Sized,
    {
        let bytes = byte_stream.slice::<Self>(MAGIC.len())?;
        match bytes == MAGIC {
            true => Ok(Self),
            false => Err(ConversionError::from_message("invalid magic number")),
        }
    }
}

impl<const MAGIC: &'static [u8]> ToBytes for Signature<MAGIC> {
    fn to_bytes(&self) -> ConversionResult<Vec<u8>> {
        Ok(MAGIC.to_vec())
    }
}

#[cfg(feature = "interface")]
impl<const MAGIC: &'static [u8], App: korangar_interface::application::Application> korangar_interface::elements::PrototypeElement<App>
    for Signature<MAGIC>
{
    fn to_element(&self, display: String) -> korangar_interface::elements::ElementCell<App> {
        std::str::from_utf8(MAGIC).unwrap().to_element(display)
    }
}
