//! An OS folder containing game assets.
use std::fs;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use super::{Archive, Writable};

pub struct FolderArchive {
    folder_path: PathBuf,
}

impl FolderArchive {
    fn os_specific_path(path: &str) -> PathBuf {
        match cfg!(target_os = "windows") {
            true => PathBuf::from(path),
            false => PathBuf::from(path.replace('\\', "/")),
        }
    }
}

impl Archive for FolderArchive {
    fn from_path(path: &Path) -> Self {
        Self {
            folder_path: PathBuf::from(path),
        }
    }

    fn get_file_by_path(&mut self, asset_path: &str) -> Option<Vec<u8>> {
        let normalized_asset_path = Self::os_specific_path(asset_path);
        let full_path = self.folder_path.join(normalized_asset_path);

        full_path.is_file().then(|| fs::read(full_path).ok()).flatten()
    }

    fn get_lua_files(&self, lua_files: &mut Vec<String>) {
        let files = WalkDir::new(&self.folder_path)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_file() && entry.path().extension().unwrap() == "lub")
            .map(|file| String::from(file.path().strip_prefix(&self.folder_path).unwrap().to_str().unwrap()));

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
