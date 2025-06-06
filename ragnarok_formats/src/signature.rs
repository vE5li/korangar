use ragnarok_bytes::{ByteReader, ByteWriter, ConversionError, ConversionResult, FixedByteSize, FromBytes, ToBytes};
use rust_state::Path;

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
    type Layouted = impl std::any::Any;
    type Return<P>
        = impl korangar_interface::element::Element<App, Layouted = Self::Layouted>
    where
        P: rust_state::Path<App, Self>;

    fn to_element<P>(self_path: P, name: String) -> Self::Return<P>
    where
        P: Path<App, Self>,
    {
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
