//! Generic file loading functionality.
#![warn(missing_docs)]

/// Error that is thrown when a file loader can't find the requested file.
#[repr(transparent)]
pub struct FileNotFoundError(String);

impl FileNotFoundError {
    /// Create a new [`FileNotFoundError`] with a given path.
    pub fn new(path: String) -> Self {
        Self(path)
    }
}

impl std::fmt::Debug for FileNotFoundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "can't find file: {}", self.0)
    }
}

/// Trait for general file loading.
pub trait FileLoader: Send + Sync + 'static {
    /// Returns the file content of the requested file.
    fn get(&self, path: &str) -> Result<Vec<u8>, FileNotFoundError>;
}
