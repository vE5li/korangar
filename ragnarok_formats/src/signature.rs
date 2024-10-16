use ragnarok_bytes::{ByteReader, ByteWriter, ConversionError, ConversionResult, FixedByteSize, FromBytes, ToBytes};

#[derive(Debug, Clone, Default)]
pub struct Signature<const MAGIC: &'static [u8]>;

impl<const MAGIC: &'static [u8]> FixedByteSize for Signature<MAGIC> {
    fn size_in_bytes() -> usize {
        MAGIC.len()
    }
}

impl<const MAGIC: &'static [u8]> FromBytes for Signature<MAGIC> {
    fn from_bytes<Meta>(byte_reader: &mut ByteReader<Meta>) -> ConversionResult<Self>
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

#[cfg(feature = "interface")]
impl<const MAGIC: &'static [u8], App: korangar_interface::application::Appli> korangar_interface::element::PrototypeElement<App>
    for Signature<MAGIC>
{
    fn to_element(self_path: impl rust_state::Path<App, Self>, name: String) -> impl korangar_interface::element::Element<App> {
        use korangar_interface::prelude::*;

        button! {
            text: name,
            event: |state: &rust_state::Context<App>, _: &mut korangar_interface::event::EventQueue<App>| {
                println!("Just a dummy for now");
            },
        }

        // std::str::from_utf8(MAGIC).unwrap().to_element(display)
    }
}
