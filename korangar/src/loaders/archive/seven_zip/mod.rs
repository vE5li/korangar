//! A 7zip file containing game assets.

mod builder;

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use blake3::Hasher;
use hashbrown::HashMap;
#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, Timer, print_debug};
use sevenz_rust2::{ArchiveReader, EncoderMethod, Password};

pub use self::builder::SevenZipArchiveBuilder;
use crate::loaders::archive::{Archive, Compression};

struct FileIndexEntry {
    compression: Compression,
    file_name: String,
}

pub struct SevenZipArchive {
    pub(crate) reader: Mutex<ArchiveReader<BufReader<File>>>,
    file_index: HashMap<String, FileIndexEntry>,
    file_path: PathBuf,
}

impl SevenZipArchive {
    pub fn file_exists(&self, asset_path: &str) -> bool {
        self.file_index.contains_key(asset_path)
    }

    pub fn file_is_compressed(&self, asset_path: &str) -> Option<Compression> {
        self.file_index.get(asset_path).map(|entry| entry.compression)
    }
}

impl Archive for SevenZipArchive {
    fn from_path(path: &Path) -> Self {
        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load game data from {}", path.display().magenta()));
        let file = File::open(path).expect("can't open archive");
        let reader = ArchiveReader::new(BufReader::new(file), Password::empty()).unwrap();
        let archive = reader.archive();

        assert!(!archive.is_solid, "7zip archives needs to be non-solid for fast file access");

        let mut compression_methods = Vec::new();
        let mut file_index = HashMap::with_capacity(archive.files.len());
        archive.files.iter().for_each(|file| {
            let name_with_backslash = file.name().replace('/', "\\").to_lowercase();

            compression_methods.clear();
            reader
                .file_compression_methods(file.name(), &mut compression_methods)
                .expect("can't read compression methods of file");

            let compression = if compression_methods.iter().any(|method| method.id() == EncoderMethod::ID_COPY) {
                Compression::Off
            } else {
                Compression::Default
            };

            file_index.insert(name_with_backslash, FileIndexEntry {
                compression,
                file_name: file.name().to_string(),
            });
        });

        #[cfg(feature = "debug")]
        timer.stop();

        Self {
            reader: Mutex::new(reader),
            file_index,
            file_path: PathBuf::from(path),
        }
    }

    fn file_exists(&self, asset_path: &str) -> bool {
        self.file_index.contains_key(asset_path)
    }

    fn get_file_by_path(&self, asset_path: &str) -> Option<Vec<u8>> {
        self.file_index
            .get(asset_path)
            .and_then(|entry| self.reader.lock().unwrap().read_file(entry.file_name.as_str()).ok())
    }

    fn get_files_with_extension(&self, files: &mut Vec<String>, extensions: &[&str]) {
        self.reader
            .lock()
            .unwrap()
            .archive()
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
