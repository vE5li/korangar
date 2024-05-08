use ragnarok_bytes::ConversionError;

use super::FileNotFoundError;

#[derive(Debug)]
pub enum LoadError {
    File(FileNotFoundError),
    Conversion(Box<ConversionError>),
    UnsupportedFormat(String),
}
