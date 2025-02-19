use cgmath::Point2;
use ragnarok_bytes::{FromBytes, ToBytes};
use ragnarok_formats::signature::Signature;
use ragnarok_formats::version::{MajorFirst, Version};

#[derive(ToBytes, FromBytes)]
pub struct TextureAtlasData {
    /// Korangar Texture Atlas
    #[new_default]
    pub signature: Signature<b"kta">,
    /// Version of the texture atlas data.
    #[version]
    pub version: Version<MajorFirst>,
    /// THe name of the texture atlas.
    pub name: String,
    /// Format of the compressed data. Currently only "0" is allowed and means
    /// "BC7".
    pub format: u32,
    /// Width of the texture atlas in pixel.
    pub width: u32,
    /// Height of the texture atlas in pixel.
    pub height: u32,
    /// The mips level count. When no mips are present, then count must be 1.
    pub mipmaps_count: u32,
    /// The hash of the input textures (sorted by asset name).
    pub hash: [u8; 32],
    /// Number of lookup entries.
    #[new_derive]
    pub lookup_count: u32,
    /// Texture lookup entries.
    #[repeating(lookup_count)]
    pub lookup: Vec<LookupEntry>,
    /// Number of allocation entries.
    #[new_derive]
    pub allocations_count: u32,
    /// Texture atlas allocations.
    #[repeating(allocations_count)]
    pub allocations: Vec<AllocationEntry>,
    /// The size of the compressed data following.
    pub compressed_data_size: u32,
}

#[derive(ToBytes, FromBytes)]
pub struct LookupEntry {
    /// The name of the asset texture.
    pub name: String,
    /// The ID of the allocation for this texture.
    pub allocation_id: u32,
    /// Marker if the texture contains pixel with an alpha value that is not
    /// 255.
    pub transparent: u32,
}

#[derive(ToBytes, FromBytes)]
pub struct AllocationEntry {
    /// ID of the allocation.
    pub id: u32,
    /// Min point of the position of the texture inside the atlas.
    pub min: Point2<u32>,
    /// Max point of the position of the texture inside the atlas.
    pub max: Point2<u32>,
}
