pub mod folder;
pub mod native;
pub mod seven_zip;

use std::path::{Path, PathBuf};

pub trait Archive: Send + Sync {
    fn from_path(path: &Path) -> Self
    where
        Self: Sized;

    /// Retrieve an asset from the Archive.
    fn get_file_by_path(&self, asset_path: &str) -> Option<Vec<u8>>;

    /// Get a list of all files with a given extension.
    fn get_files_with_extension(&self, files: &mut Vec<String>, extension: &str);

    /// Hashes the archive with the given hasher.
    fn hash(&self, hasher: &mut blake3::Hasher);
}

pub enum ArchiveType {
    Folder,
    Native,
    SevenZip,
}

/// Type of compression to apply.
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum Compression {
    /// No compression.
    No,
    /// Slow compression allowed.
    Slow,
    /// Fast compression requested.
    Fast,
}

/// A common trait to all writable archives.
pub trait Writable {
    fn add_file(&mut self, path: &str, asset: Vec<u8>, compression: Compression);
    fn finish(&mut self) -> Result<(), std::io::Error>;
}

/// Converts a RO internal path to the OS specific path.
pub fn os_specific_path(path: &str) -> PathBuf {
    match cfg!(target_os = "windows") {
        true => PathBuf::from(path),
        false => PathBuf::from(path.replace('\\', "/")),
    }
}
