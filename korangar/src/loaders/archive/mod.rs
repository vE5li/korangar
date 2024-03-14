pub mod folder;
pub mod native;

use std::path::Path;

pub trait Archive {
    fn from_path(path: &Path) -> Self
    where
        Self: Sized;

    /// Retrieve an asset from the Archive
    fn get_file_by_path(&mut self, asset_path: &str) -> Option<Vec<u8>>;

    /// Get a list of all Lua files
    fn get_lua_files(&self, lua_files: &mut Vec<String>);
}

pub enum ArchiveType {
    Folder,
    Native,
}

/// A common trait to all writable archives
pub trait Writable {
    fn create(&mut self) {}

    fn add_file(&mut self, path: &str, asset: Vec<u8>);

    fn save(&self) {}
}
