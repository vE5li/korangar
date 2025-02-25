//! An OS folder containing game assets.
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::{Error, Read};
use std::path::{Path, PathBuf};

use blake3::Hasher;
use flate2::bufread::{GzDecoder, GzEncoder};
#[cfg(feature = "debug")]
use korangar_debug::logging::print_debug;
use walkdir::WalkDir;

use super::{Archive, Compression, Writable, os_specific_path};

pub struct FolderArchive {
    folder_path: PathBuf,
    /// In the native archives, file names are case-insensitive and use '\' as a
    /// separator, but our file system might not. This mapping lets us do a
    /// lookup from a unified format to the actual file name in the file system.
    ///
    /// Example:
    /// ```
    /// "texture\\data\\angel.str" -> texture/data/Angel.str
    /// ```
    file_mapping: HashMap<String, PathBuf>,
}

impl FolderArchive {
    /// Load the file mapping of a given directory.
    fn load_mapping(directory: &PathBuf) -> HashMap<String, PathBuf> {
        WalkDir::new(directory)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_file())
            .map(|file| {
                let mut asset_path = file
                    .path()
                    .strip_prefix(directory)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .replace('/', "\\")
                    .to_lowercase();

                if asset_path.ends_with(".gz") {
                    asset_path = asset_path.strip_suffix(".gz").unwrap().to_string();
                }

                (asset_path, file.into_path())
            })
            .collect()
    }

    fn compress_gz(mut full_path: PathBuf, encoder: &mut GzEncoder<&[u8]>) -> (PathBuf, Vec<u8>) {
        let mut compressed = Vec::default();
        encoder.read_to_end(&mut compressed).unwrap();

        let extension = full_path.extension().unwrap_or_default().to_string_lossy().into_owned();

        let compressed_extension = format!("{}.gz", extension);
        full_path.set_extension(compressed_extension);

        (full_path, compressed)
    }
}

impl Archive for FolderArchive {
    fn from_path(path: &Path) -> Self {
        let folder_path = PathBuf::from(path);
        let file_mapping = Self::load_mapping(&folder_path);

        Self { folder_path, file_mapping }
    }

    fn get_file_by_path(&self, asset_path: &str) -> Option<Vec<u8>> {
        self.file_mapping.get(asset_path).and_then(|file_path| {
            fs::read(file_path)
                .map(|file_data| {
                    if file_path.extension().unwrap_or_default() == "gz" {
                        let mut decoder = GzDecoder::new(file_data.as_slice());
                        let mut decompressed = Vec::new();
                        decoder.read_to_end(&mut decompressed).unwrap();
                        decompressed
                    } else {
                        file_data
                    }
                })
                .ok()
        })
    }

    fn get_files_with_extension(&self, files: &mut Vec<String>, extension: &str) {
        let found_files = self.file_mapping.keys().filter(|file_name| file_name.ends_with(extension)).cloned();
        files.extend(found_files);
    }

    fn hash(&self, hasher: &mut Hasher) {
        let mut files: Vec<PathBuf> = self.file_mapping.values().cloned().collect();
        files.sort();
        files.iter().for_each(|file_path| match File::open(file_path) {
            Ok(file) => {
                if let Err(_err) = hasher.update_reader(&file) {
                    #[cfg(feature = "debug")]
                    print_debug!("Can't hash archive file `{:?}`: {:?}", file_path, _err);
                }
            }
            Err(_err) => {
                #[cfg(feature = "debug")]
                print_debug!("Can't open archive file `{:?}`: {:?}", file_path, _err);
            }
        });
    }
}

impl Writable for FolderArchive {
    fn add_file(&mut self, file_path: &str, file_data: Vec<u8>, compression: Compression) {
        let normalized_asset_path = os_specific_path(file_path);
        let full_path = self.folder_path.join(normalized_asset_path);

        // Create parent directories if needed
        if let Some(parent) = full_path.parent() {
            if !parent.exists() {
                if let Err(err) = fs::create_dir_all(parent) {
                    panic!("error creating directories: {}", err);
                }
            }
        }

        let (path, data) = match compression {
            Compression::No => (full_path, file_data),
            Compression::Slow => {
                let mut encoder = GzEncoder::new(file_data.as_slice(), flate2::Compression::best());
                Self::compress_gz(full_path, &mut encoder)
            }
            Compression::Fast => {
                let mut encoder = GzEncoder::new(file_data.as_slice(), flate2::Compression::fast());
                Self::compress_gz(full_path, &mut encoder)
            }
        };

        fs::write(&path, data).unwrap_or_else(|_| panic!("error writing to file {}", path.display()));

        self.file_mapping.insert(file_path.to_string(), path);
    }

    fn finish(&mut self) -> Result<(), Error> {
        Ok(())
    }
}
