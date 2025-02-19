//! Implements a writable instance of a GRF File.
//!
//! This way, we can provide a temporal storage to files before the final write
//! occurs while keeping it outside the
//! [`NativeArchive`](super::NativeArchive) implementation

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use flate2::bufread::ZlibEncoder;
use flate2::Compression;
use ragnarok_bytes::{FixedByteSize, ToBytes};
use ragnarok_formats::archive::{AssetTable, FileTableRow, Header};

use super::FileTable;
use crate::loaders::archive::Writable;

struct FileTableEntry {
    path: String,
    compress: bool,
    asset_data: Vec<u8>,
}

pub struct NativeArchiveBuilder {
    os_file_path: PathBuf,
    archive_entries: Vec<FileTableEntry>,
}

impl NativeArchiveBuilder {
    pub fn from_path(path: &Path) -> Self {
        Self {
            os_file_path: PathBuf::from(path),
            archive_entries: Vec::new(),
        }
    }
}

impl Writable for NativeArchiveBuilder {
    fn add_file(&mut self, path: &str, asset_data: Vec<u8>, compress: bool) {
        self.archive_entries.push(FileTableEntry {
            path: path.to_string(),
            compress,
            asset_data,
        });
    }

    fn finish(&mut self) -> Result<(), std::io::Error> {
        let file = File::create(self.os_file_path.as_path())?;
        let mut file_writer = BufWriter::new(file);
        let mut file_table = FileTable::new();

        let dummy_header_bytes = vec![0; Header::size_in_bytes()];
        file_writer.write_all(&dummy_header_bytes)?;

        let mut offset = 0;

        for entry in self.archive_entries.drain(..) {
            add_asset_to_file_table(
                &mut file_writer,
                &mut offset,
                &mut file_table,
                &entry.path,
                entry.asset_data,
                entry.compress,
            );
        }

        let mut file_table_data = Vec::new();

        for file_information in file_table.values() {
            file_table_data.extend(file_information.to_bytes().unwrap());
        }

        let mut encoder = ZlibEncoder::new(file_table_data.as_slice(), Compression::fast());
        let mut compressed = Vec::default();
        encoder.read_to_end(&mut compressed)?;

        let asset_table = AssetTable {
            compressed_size: compressed.len() as u32,
            uncompressed_size: file_table_data.len() as u32,
        };

        let file_table_bytes = asset_table.to_bytes().unwrap();
        file_writer.write_all(&file_table_bytes)?;
        file_writer.write_all(&compressed)?;

        let reserved_files = 0;
        let raw_file_count = file_table.len() as u32 + 7;
        let version = 0x200;
        let file_header_bytes = Header::new(offset, reserved_files, raw_file_count, version).to_bytes().unwrap();

        file_writer.seek(SeekFrom::Start(0))?;
        file_writer.write_all(&file_header_bytes)?;
        file_writer.flush()?;

        Ok(())
    }
}

fn add_asset_to_file_table(
    file_writer: &mut BufWriter<File>,
    offset: &mut u32,
    file_table: &mut HashMap<String, FileTableRow>,
    path: &str,
    data: Vec<u8>,
    compress: bool,
) {
    let uncompressed_size = data.len() as u32;

    let data = match compress {
        true => {
            let mut encoder = ZlibEncoder::new(data.as_slice(), Compression::fast());
            let mut compressed = Vec::default();
            encoder.read_to_end(&mut compressed).expect("can't compress asset data");
            compressed
        }
        false => data,
    };

    let compressed_size = data.len() as u32;
    let compressed_size_aligned = compressed_size;
    let flags = 1;

    let file_information = FileTableRow {
        file_name: path.to_string(),
        compressed_size,
        compressed_size_aligned,
        uncompressed_size,
        flags,
        offset: *offset,
    };
    *offset = offset.checked_add(data.len() as u32).expect("offset overflow");

    file_table.insert(path.to_string(), file_information);
    file_writer.write_all(&data).unwrap()
}
