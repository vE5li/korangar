use ragnarok_bytes::{
    ByteConvertable, ByteReader, ByteWriter, ConversionError, ConversionResult, ConversionResultExt, FromBytes, FromBytesExt, ToBytes,
};

use crate::signature::Signature;
use crate::version::{MinorFirst, Version, VersionMetadata};

#[derive(Clone, Debug)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct PaletteImageData {
    pub width: u16,
    pub height: u16,
    pub data: EncodedData,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct EncodedData(pub Vec<u8>);

impl FromBytes for PaletteImageData {
    fn from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self>
    where
        Self: Sized,
    {
        let width = u16::from_bytes(byte_reader).trace::<Self>()?;
        let height = u16::from_bytes(byte_reader).trace::<Self>()?;

        let data = match width as usize * height as usize {
            0 => Vec::new(),
            image_size
                if byte_reader
                    .get_metadata::<Self, dyn VersionMetadata>()?
                    .get_version()
                    .ok_or(ConversionError::from_message("version not set"))?
                    .below(2, 1) =>
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
                        if encoded == 0 {
                            return Err(ConversionError::from_message("truncated palette RLE escape"));
                        }
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct RgbaImageData {
    pub width: u16,
    pub height: u16,
    #[repeating_expr(width as usize * height as usize * 4)]
    pub data: Vec<u8>,
}

#[derive(Copy, Clone, Debug, Default, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct PaletteColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub reserved: u8,
}

#[derive(Clone, Debug, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct Palette {
    pub colors: [PaletteColor; 256],
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct SpriteData {
    pub signature: Signature<b"SP">,
    pub version: Version<MinorFirst>,
    pub palette_image_count: u16,
    pub rgba_image_count: Option<u16>,
    pub palette_image_data: Vec<PaletteImageData>,
    pub rgba_image_data: Vec<RgbaImageData>,
    pub palette: Option<Palette>,
}

impl SpriteData {
    pub fn new(
        version: Version<MinorFirst>,
        palette_image_data: Vec<PaletteImageData>,
        rgba_image_data: Vec<RgbaImageData>,
        palette: Option<Palette>,
    ) -> Self {
        let palette_image_count = palette_image_data.len() as u16;
        let rgba_image_count = crate::version::InternalVersion::from(version)
            .equals_or_above(1, 2)
            .then_some(rgba_image_data.len() as u16);
        Self {
            signature: Signature,
            version,
            palette_image_count,
            rgba_image_count,
            palette_image_data,
            rgba_image_data,
            palette,
        }
    }
}

impl FromBytes for SpriteData {
    fn from_bytes(reader: &mut ByteReader) -> ConversionResult<Self> {
        let signature = Signature::from_bytes(reader).trace::<Self>()?;
        let version = Version::<MinorFirst>::from_bytes(reader).trace::<Self>()?;
        let internal = crate::version::InternalVersion::from(version);
        reader.get_metadata_mut::<Self, dyn VersionMetadata>()?.set_version(internal);
        let palette_image_count = u16::from_bytes(reader).trace::<Self>()?;
        let rgba_image_count = if internal.equals_or_above(1, 2) {
            Some(u16::from_bytes(reader).trace::<Self>()?)
        } else {
            None
        };
        let palette_image_data = (0..palette_image_count)
            .map(|_| PaletteImageData::from_bytes(reader))
            .collect::<ConversionResult<Vec<_>>>()?;
        let rgba_image_data = (0..rgba_image_count.unwrap_or_default())
            .map(|_| RgbaImageData::from_bytes(reader))
            .collect::<ConversionResult<Vec<_>>>()?;
        // RGBA-only SPRs may omit the palette entirely. If a palette is present,
        // still read it strictly (a truncated palette must not be ignored).
        let palette = if internal.equals_or_above(1, 1) && (palette_image_count > 0 || !reader.is_empty()) {
            Some(Palette::from_bytes(reader).trace::<Self>()?)
        } else {
            None
        };
        if palette_image_count > 0 && palette.is_none() {
            return Err(ConversionError::from_message("indexed SPR requires a palette"));
        }
        Ok(Self {
            signature,
            version,
            palette_image_count,
            rgba_image_count,
            palette_image_data,
            rgba_image_data,
            palette,
        })
    }
}

impl ToBytes for SpriteData {
    fn to_bytes(&self, writer: &mut ByteWriter) -> ConversionResult<usize> {
        // Indexed image encoding was already unsupported by PaletteImageData.
        // Return an error instead of panicking or publishing inconsistent counts.
        if !self.palette_image_data.is_empty() {
            return Err(ConversionError::from_message("indexed SPR encoding is unsupported"));
        }
        let start = writer.len();
        self.signature.to_bytes(writer)?;
        self.version.to_bytes(writer)?;
        0u16.to_bytes(writer)?;
        if crate::version::InternalVersion::from(self.version).equals_or_above(1, 2) {
            let count = u16::try_from(self.rgba_image_data.len()).map_err(|_| ConversionError::from_message("too many SPR images"))?;
            count.to_bytes(writer)?;
        }
        for image in &self.rgba_image_data {
            image.to_bytes(writer)?;
        }
        if let Some(palette) = &self.palette {
            palette.to_bytes(writer)?;
        }
        Ok(writer.len() - start)
    }
}

#[cfg(test)]
mod sprite_parity_tests {
    use super::*;
    use crate::version::GenericFormatMetadata;
    fn rgba_only() -> Vec<u8> {
        vec![b'S', b'P', 1, 2, 0, 0, 1, 0, 1, 0, 1, 0, 255, 10, 20, 30]
    }
    #[test]
    fn rgba_only_sprite_without_palette_parses_and_roundtrips() {
        let bytes = rgba_only();
        let mut reader = ByteReader::with_default_metadata::<GenericFormatMetadata>(&bytes);
        let sprite = SpriteData::from_bytes(&mut reader).unwrap();
        assert!(reader.is_empty());
        assert!(sprite.palette.is_none());
        assert_eq!(sprite.rgba_image_data.len(), 1);
        let mut writer = ByteWriter::new();
        sprite.to_bytes(&mut writer).unwrap();
        assert_eq!(writer.as_slice(), bytes);
    }
    #[test]
    fn optional_palette_must_be_complete_if_present() {
        let mut bytes = rgba_only();
        bytes.push(1);
        assert!(SpriteData::from_bytes(&mut ByteReader::with_default_metadata::<GenericFormatMetadata>(&bytes)).is_err());
        bytes.resize(16 + 1024, 0);
        assert!(
            SpriteData::from_bytes(&mut ByteReader::with_default_metadata::<GenericFormatMetadata>(&bytes))
                .unwrap()
                .palette
                .is_some()
        );
    }
    #[test]
    fn legacy_indexed_sprite_and_malformed_rle_are_handled() {
        let mut legacy = vec![b'S', b'P', 1, 1, 1, 0, 1, 0, 1, 0, 5];
        legacy.resize(11 + 1024, 0);
        let sprite = SpriteData::from_bytes(&mut ByteReader::with_default_metadata::<GenericFormatMetadata>(&legacy)).unwrap();
        assert_eq!(sprite.palette_image_data[0].data.0, vec![5]);
        let bad = vec![b'S', b'P', 1, 2, 1, 0, 0, 0, 1, 0, 1, 0, 1, 0, 0, 1];
        assert!(SpriteData::from_bytes(&mut ByteReader::with_default_metadata::<GenericFormatMetadata>(&bad)).is_err());
    }
    #[test]
    fn indexed_image_requires_complete_palette_and_pixels() {
        let mut bytes = vec![b'S', b'P', 1, 2, 1, 0, 0, 0, 1, 0, 1, 0, 1, 0, 5];
        assert!(SpriteData::from_bytes(&mut ByteReader::with_default_metadata::<GenericFormatMetadata>(&bytes)).is_err());
        bytes.resize(15 + 1024, 0);
        let mut reader = ByteReader::with_default_metadata::<GenericFormatMetadata>(&bytes);
        assert_eq!(SpriteData::from_bytes(&mut reader).unwrap().palette_image_data[0].data.0, vec![
            5
        ]);
        assert!(reader.is_empty());
        assert!(SpriteData::from_bytes(&mut ByteReader::with_default_metadata::<GenericFormatMetadata>(&bytes[..10])).is_err());
    }
}
