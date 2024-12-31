//! Implements a writable instance of a GRF File
//! This way, we can provide a temporal storage to files before the final write
//! occurs while keeping it outside the
//! [`NativeArchive`](super::NativeArchive) implementation

use std::io::Read;
use std::path::{Path, PathBuf};

use flate2::bufread::ZlibEncoder;
use flate2::Compression;
use ragnarok_bytes::ToBytes;
use ragnarok_formats::archive::{AssetTable, FileTableRow, Header};

use super::FileTable;
use crate::loaders::archive::Writable;

pub struct NativeArchiveBuilder {
    os_file_path: PathBuf,
    file_table: FileTable,
    data: Vec<u8>,
}

impl NativeArchiveBuilder {
    pub fn from_path(path: &Path) -> Self {
        Self {
            os_file_path: PathBuf::from(path),
            file_table: FileTable::new(),
            data: Vec::new(),
        }
    }
}

impl Writable for NativeArchiveBuilder {
    fn add_file(&mut self, path: &str, asset: Vec<u8>) {
        let mut encoder = ZlibEncoder::new(asset.as_slice(), Compression::default());
        let mut compressed = Vec::default();
        encoder.read_to_end(&mut compressed).expect("can't compress asset");

        let compressed_size = compressed.len() as u32;
        let compressed_size_aligned = compressed_size;
        let uncompressed_size = asset.len() as u32;
        let flags = 1;
        let offset = self.data.len() as u32;

        let file_information = FileTableRow {
            file_name: String::from(path),
            compressed_size,
            compressed_size_aligned,
            uncompressed_size,
            flags,
            offset,
        };

        self.data.extend_from_slice(&compressed);
        self.file_table.insert(String::from(path), file_information);
    }

    fn save(&self) {
        let file_table_offset = self.data.len() as u32;
        let reserved_files = 0;
        let raw_file_count = self.file_table.len() as u32 + 7;
        let version = 0x200;
        let file_header = Header::new(file_table_offset, reserved_files, raw_file_count, version);

        let mut bytes = file_header.to_bytes().unwrap();
        bytes.extend_from_slice(&self.data);

        let mut file_table_data = Vec::new();

        for file_information in self.file_table.values() {
            file_table_data.extend(file_information.to_bytes().unwrap());
        }

        let mut encoder = ZlibEncoder::new(file_table_data.as_slice(), Compression::default());
        let mut compressed = Vec::default();
        encoder.read_to_end(&mut compressed).expect("can't compress file information");

        let file_table = AssetTable {
            compressed_size: compressed.len() as u32,
            uncompressed_size: file_table_data.len() as u32,
        };

        bytes.extend(file_table.to_bytes().unwrap());
        bytes.extend(compressed);

        std::fs::write(&self.os_file_path, bytes).expect("unable to write file");
    }
}
