//! A 7zip file containing game assets.

mod builder;

use std::cell::RefCell;
use std::fs::File;
use std::io::{Seek, SeekFrom};
use std::path::{Path, PathBuf};

use blake3::Hasher;
use hashbrown::HashMap;
#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, Timer, print_debug};
use sevenz_rust2::BlockDecoder;

pub use self::builder::SevenZipArchiveBuilder;
use crate::loaders::archive::{Archive, Compression};

const MB_1: u64 = 1024 * 1024;
const MB_4: u64 = 4 * MB_1;
const MB_8: u64 = 8 * MB_1;

thread_local! {
    static FILE_CACHE: RefCell<HashMap<PathBuf, File>> = RefCell::new(HashMap::new());
}

struct FileIndexEntry {
    compression: Compression,
    file_crc: u64,
    file_size: u64,
    block_index: usize,
}

pub struct SevenZipArchive {
    archive: sevenz_rust2::Archive,
    password: sevenz_rust2::Password,
    file_lookup: HashMap<String, FileIndexEntry>,
    file_path: PathBuf,
}

impl SevenZipArchive {
    pub fn file_exists(&self, asset_path: &str) -> bool {
        self.file_lookup.contains_key(asset_path)
    }

    pub fn file_is_compressed(&self, asset_path: &str) -> Option<Compression> {
        self.file_lookup.get(asset_path).map(|entry| entry.compression)
    }
}

impl Archive for SevenZipArchive {
    fn from_path(path: &Path) -> Self {
        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load game data from {}", path.display().magenta()));
        let mut archive_file = File::open(path).expect("can't open archive");
        let password = sevenz_rust2::Password::empty();

        let archive = sevenz_rust2::Archive::read(&mut archive_file, &password).expect("can't read archive");

        assert!(!archive.is_solid, "7zip archives needs to be non-solid for fast file access");

        let mut file_lookup = HashMap::with_capacity(archive.files.len());

        for (file_index, file) in archive.files.iter().enumerate() {
            let Some(block_index) = archive.stream_map.file_block_index[file_index] else {
                continue;
            };

            let name_with_backslash = file.name().replace('/', "\\").to_lowercase();

            let block = &archive.blocks[block_index];

            let compression = match block
                .coders
                .iter()
                .any(|coder| coder.encoder_method_id() == sevenz_rust2::EncoderMethod::ID_COPY)
            {
                true => Compression::Off,
                false => Compression::Default,
            };

            file_lookup.insert(name_with_backslash, FileIndexEntry {
                compression,
                file_crc: file.crc,
                file_size: file.size,
                block_index,
            });
        }

        #[cfg(feature = "debug")]
        timer.stop();

        Self {
            archive,
            password,
            file_lookup,
            file_path: PathBuf::from(path),
        }
    }

    fn file_exists(&self, asset_path: &str) -> bool {
        self.file_lookup.contains_key(asset_path)
    }

    fn get_file_by_path(&self, asset_path: &str) -> Option<Vec<u8>> {
        let file_entry = self.file_lookup.get(asset_path)?;

        FILE_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();

            let archive_file = cache
                .entry(self.file_path.clone())
                .or_insert_with(|| File::open(&self.file_path).expect("can't open archive file"));
            archive_file.seek(SeekFrom::Start(0)).expect("can't seek to file start");

            let thread_count = match file_entry.compression {
                Compression::Default if file_entry.file_size > MB_8 => 8,
                Compression::Default if file_entry.file_size > MB_4 => 4,
                Compression::Default if file_entry.file_size > MB_1 => 2,
                _ => 1,
            };

            let block_decoder = BlockDecoder::new(
                thread_count,
                file_entry.block_index,
                &self.archive,
                &self.password,
                archive_file,
            );

            let mut data = vec![0; file_entry.file_size as usize];
            let mut found = false;

            block_decoder
                .for_each_entries(&mut |entry, reader| {
                    if entry.crc == file_entry.file_crc && entry.size == file_entry.file_size {
                        let _ = reader.read_exact(&mut data);
                        found = true;
                    }
                    Ok(false)
                })
                .expect("could not read entry");

            match found {
                true => Some(data),
                false => None,
            }
        })
    }

    fn get_files_with_extension(&self, files: &mut Vec<String>, extensions: &[&str]) {
        self.archive
            .files
            .iter()
            .filter(|file| {
                extensions
                    .iter()
                    .any(|extension| !file.is_directory() && file.name().ends_with(extension))
            })
            .for_each(|file| {
                let name_with_backslash = file.name().replace('/', "\\").to_lowercase();
                files.push(name_with_backslash)
            })
    }

    fn hash(&self, hasher: &mut Hasher) {
        let file = File::open(&self.file_path).unwrap();
        if let Err(_err) = hasher.update_reader(&file) {
            #[cfg(feature = "debug")]
            print_debug!("Can't hash ZIP archive: {:?}", _err);
        }
    }
}
