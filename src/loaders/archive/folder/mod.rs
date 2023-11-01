//! An OS folder containing game assets.
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use super::{Archive, Writable};

pub struct FolderArchive {
    folder_path: PathBuf,
    /// In the native archives, file names are case insensitive and use '\' as a
    /// separator, but our file system might not. This mapping let's us do a
    /// lookup from a unified format to the actual file name in the file system.
    ///
    /// Example:
    /// ```
    /// "texture\\data\\angel.str" -> texture/data/Angel.str
    /// ```
    file_mapping: HashMap<String, PathBuf>,
}

impl FolderArchive {
    fn os_specific_path(path: &str) -> PathBuf {
        match cfg!(target_os = "windows") {
            true => PathBuf::from(path),
            false => PathBuf::from(path.replace('\\', "/")),
        }
    }

    /// Load the file mapping of a given directory.
    fn load_mapping(directory: &PathBuf) -> HashMap<String, PathBuf> {
        WalkDir::new(directory)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_file())
            .map(|file| {
                let asset_path = file
                    .path()
                    .strip_prefix(directory)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .replace('/', "\\")
                    .to_lowercase();

                (asset_path, file.into_path())
            })
            .collect()
    }
}

impl Archive for FolderArchive {
    fn from_path(path: &Path) -> Self {
        let folder_path = PathBuf::from(path);
        let file_mapping = Self::load_mapping(&folder_path);

        Self { folder_path, file_mapping }
    }

    fn get_file_by_path(&mut self, asset_path: &str) -> Option<Vec<u8>> {
        self.file_mapping.get(asset_path).and_then(|file_path| fs::read(file_path).ok())
    }

    fn get_lua_files(&self, lua_files: &mut Vec<String>) {
        let files = self
            .file_mapping
            .keys()
            .filter(|file_name| file_name.ends_with(".lub"))
            .map(|file_name| file_name.clone());

        lua_files.extend(files);
    }
}

impl Writable for FolderArchive {
    fn create(&mut self) {
        fs::create_dir_all(&self.folder_path)
            .unwrap_or_else(|_| panic!("error creating folder {} for FolderArchive", self.folder_path.display()));
    }

    fn add_file(&mut self, file_path: &str, file_data: Vec<u8>) {
        let normalized_asset_path = Self::os_specific_path(file_path);
        let full_path = self.folder_path.join(normalized_asset_path);

        // Create parent directories if needed
        if let Some(parent) = full_path.parent() {
            if !parent.exists() {
                if let Err(err) = fs::create_dir_all(parent) {
                    panic!("error creating directories: {}", err);
                }
            }
        }

        // Write file contents to the file
        fs::write(&full_path, file_data).unwrap_or_else(|_| panic!("error writing to file {}", full_path.display()));
    }
}
