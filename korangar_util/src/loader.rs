/// Error that is thrown when a file loader can't find the requested file.
#[derive(Debug)]
#[repr(transparent)]
pub struct FileNotFoundError(String);

impl FileNotFoundError {
    /// Create a new [`FileNotFoundError`] with a given path.
    pub fn new(path: String) -> Self {
        Self(path)
    }
}

/// Trait for general file loading.
pub trait FileLoader: Send + Sync + 'static {
    /// Returns the file content of the requested file.
    fn get(&self, path: &str) -> Result<Vec<u8>, FileNotFoundError>;
}
