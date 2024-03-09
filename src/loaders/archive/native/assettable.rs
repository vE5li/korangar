use derive_new::new;
use ragnarok_procedural::{ByteConvertable, FixedByteSize};

/// Stores the table of files the parent GRF is holding.
#[derive(Clone, ByteConvertable, FixedByteSize, new)]
pub(super) struct AssetTable {
    compressed_size: u32,
    uncompressed_size: u32,
}

impl AssetTable {
    pub(super) fn get_compressed_size(&self) -> usize {
        self.compressed_size as usize
    }
}
