//! A GRF file containing game assets.
mod assettable;
mod builder;
mod filetablerow;
mod header;

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use yazi::{decompress, Format};

use self::assettable::AssetTable;
pub use self::builder::NativeArchiveBuilder;
use self::filetablerow::FileTableRow;
use self::header::Header;
#[cfg(feature = "debug")]
use crate::debug::*;
use crate::loaders::archive::Archive;
use crate::loaders::{ByteConvertable, ByteStream, FixedByteSize};

/// Represents a GRF file. GRF Files are an archive to store game assets.
/// Each GRF contains a [`ArchiveHeader`] with metadata (ammount of files, size,
/// etc.) and a table [`AssetTable`] with information ([`AssetInformation`])
/// about individual assets.
type FileTable = HashMap<String, FileTableRow>;

pub struct NativeArchive {
    file_table: FileTable,
    os_file_handler: File,
}

const MAGIC_BYTES: &[u8] = b"Master of Magic\0";
const UNPACKED_SIZE_OF_MAGIC_STRING: usize = MAGIC_BYTES.len();
const UNPACKED_SIZE_OF_ARCHIVEHEADER: usize = Header::size_in_bytes();
const UNPACKED_SIZE_OF_FILETABLE: usize = AssetTable::size_in_bytes();

impl Archive for NativeArchive {
    // Keeping the convenince of using [`loaders::stream::ByteStream`]
    /// while being able to read without buffering all file.
    fn from_path(path: &Path) -> Self {
        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load game data from {MAGENTA}{0}{NONE}", path.display()));
        let mut file = File::open(path).unwrap();

        let mut magic_number_buffer = [0u8; UNPACKED_SIZE_OF_MAGIC_STRING];
        file.read_exact(&mut magic_number_buffer).unwrap();

        let mut file_header_buffer = [0u8; UNPACKED_SIZE_OF_ARCHIVEHEADER];
        file.read_exact(&mut file_header_buffer).unwrap();
        let file_header = Header::from_bytes(&mut ByteStream::new(&file_header_buffer), None);
        file_header.validate_version();

        let _ = file.seek(SeekFrom::Current(file_header.get_file_table_offset() as i64)).unwrap();
        let mut file_table_buffer = [0u8; UNPACKED_SIZE_OF_FILETABLE];

        file.read_exact(&mut file_table_buffer).unwrap();
        let file_table = AssetTable::from_bytes(&mut ByteStream::new(&file_table_buffer), None);

        let mut compressed_file_table_buffer = vec![0u8; file_table.get_compressed_size()];
        file.read_exact(&mut compressed_file_table_buffer).unwrap();
        let (decompressed, _checksum) = decompress(&compressed_file_table_buffer, Format::Zlib).unwrap();

        let file_count = file_header.get_file_count();

        let mut file_table_byte_stream = ByteStream::new(&decompressed);
        let mut assets = HashMap::with_capacity(file_count);

        for _index in 0..file_count {
            let file_information = FileTableRow::from_bytes(&mut file_table_byte_stream, None);
            let file_name = file_information.file_name.to_lowercase();

            assets.insert(file_name, file_information);
        }

        #[cfg(feature = "debug")]
        timer.stop();

        // TODO: only take 64..? bytes so that loaded game archives can be extended
        // aswell
        Self {
            file_table: assets,
            os_file_handler: file,
        }
    }

    /// Returns an asset from the archive.
    /// Checks if the file is cached and if so, returns it from memory.
    /// If not, read from the [`GameArchive`]
    fn get_file_by_path(&mut self, path: &str) -> Option<Vec<u8>> {
        self.file_table.get(path).and_then(|file_information| {
            let mut compressed_file_buffer = vec![0u8; file_information.compressed_size_aligned as usize];

            // TODO: Figure out what the GRF_FLAG_MIXCRYPT flag actually means and load the
            // file correctly
            if file_information.flags > 1 {
                return None;
            }

            let position = file_information.offset as u64 + UNPACKED_SIZE_OF_MAGIC_STRING as u64 + UNPACKED_SIZE_OF_ARCHIVEHEADER as u64;
            self.os_file_handler.seek(SeekFrom::Start(position)).unwrap();
            self.os_file_handler.read_exact(&mut compressed_file_buffer).unwrap();

            let (uncompressed_file_buffer, _checksum) = decompress(&compressed_file_buffer, Format::Zlib).unwrap();

            Some(uncompressed_file_buffer)
        })
    }

    fn get_lua_files(&self, lua_files: &mut Vec<String>) {
        let files = self
            .file_table
            .iter()
            .filter(|(file_name, row)| file_name.ends_with(".lub") && row.flags == 0x01)
            .map(|(file_name, _)| file_name.clone());

        lua_files.extend(files);
    }
}
