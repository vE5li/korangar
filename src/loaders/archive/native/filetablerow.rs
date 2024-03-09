use ragnarok_procedural::ByteConvertable;

/// Represents file information about each of the files stored in the GRF.
#[derive(Clone, Debug, ByteConvertable)]
pub(super) struct FileTableRow {
    pub file_name: String,
    pub compressed_size: u32,
    pub compressed_size_aligned: u32,
    pub uncompressed_size: u32,
    pub flags: u8,
    pub offset: u32,
}
