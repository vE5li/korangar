use ragnarok_bytes::{
    ByteConvertable, ByteReader, ByteWriter, ConversionError, ConversionResult, ConversionResultExt, FromBytes, FromBytesExt, ToBytes,
};

use crate::signature::Signature;
use crate::version::{InternalVersion, MinorFirst, Version};

#[derive(Clone, Debug)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct PaletteImageData {
    pub width: u16,
    pub height: u16,
    pub data: EncodedData,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct EncodedData(pub Vec<u8>);

impl FromBytes for PaletteImageData {
    fn from_bytes<Meta>(byte_reader: &mut ByteReader<Meta>) -> ConversionResult<Self>
    where
        Self: Sized,
    {
        let width = u16::from_bytes(byte_reader).trace::<Self>()?;
        let height = u16::from_bytes(byte_reader).trace::<Self>()?;

        let data = match width as usize * height as usize {
            0 => Vec::new(),
            image_size
                if byte_reader
                    .get_metadata::<Self, Option<InternalVersion>>()?
                    .ok_or(ConversionError::from_message("version not set"))?
                    .smaller(2, 1) =>
            {
                Vec::from_n_bytes(byte_reader, image_size).trace::<Self>()?
            }
            image_size => {
                let mut data = vec![0; image_size];
                let mut encoded = u16::from_bytes(byte_reader).trace::<Self>()?;
                let mut next = 0;

                while next < image_size && encoded > 0 {
                    let byte = byte_reader.byte::<Self>()?;
                    encoded -= 1;

                    if byte == 0 {
                        let length = usize::max(byte_reader.byte::<Self>()? as usize, 1);
                        encoded -= 1;

                        if next + length > image_size {
                            return Err(ConversionError::from_message("too much data encoded in palette image"));
                        }

                        next += length;
                    } else {
                        data[next] = byte;
                        next += 1;
                    }
                }

                if next != image_size || encoded > 0 {
                    return Err(ConversionError::from_message("badly encoded palette image"));
                }

                data
            }
        };

        Ok(Self {
            width,
            height,
            data: EncodedData(data),
        })
    }
}

impl ToBytes for PaletteImageData {
    fn to_bytes(&self, _byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        panic!("PalletteImageData can not be serialized currently because it depends on a version requirement");
    }
}

#[derive(Clone, Debug, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct RgbaImageData {
    pub width: u16,
    pub height: u16,
    #[repeating_expr(width as usize * height as usize * 4)]
    pub data: Vec<u8>,
}

#[derive(Copy, Clone, Debug, Default, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct PaletteColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub reserved: u8,
}

#[derive(Clone, Debug, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct Palette {
    pub colors: [PaletteColor; 256],
}

#[derive(Clone, Debug, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct SpriteData {
    #[new_default]
    pub signature: Signature<b"SP">,
    #[version]
    pub version: Version<MinorFirst>,
    #[new_derive]
    pub palette_image_count: u16,
    #[version_equals_or_above(1, 2)]
    #[new_derive]
    pub rgba_image_count: Option<u16>,
    #[repeating(palette_image_count)]
    pub palette_image_data: Vec<PaletteImageData>,
    #[repeating_option(rgba_image_count)]
    pub rgba_image_data: Vec<RgbaImageData>,
    #[version_equals_or_above(1, 1)]
    pub palette: Option<Palette>,
}
