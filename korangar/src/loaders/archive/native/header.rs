use derive_new::new;
use ragnarok_bytes::{ByteConvertable, FixedByteSize};

/// Represents the Header of the GRF file.
#[derive(Clone, ByteConvertable, FixedByteSize, new)]
pub(super) struct Header {
    #[new(default)]
    encryption: [u8; 14],
    file_table_offset: u32,
    reserved_files: u32,
    file_count: u32,
    version: u32,
}

impl Header {
    pub fn validate_version(&self) {
        assert_eq!(self.version, 0x200, "invalid grf version");
    }

    pub fn get_file_table_offset(&self) -> usize {
        self.file_table_offset as usize
    }

    pub fn get_file_count(&self) -> usize {
        (self.file_count - self.reserved_files) as usize - 7
    }
}
