use korangar_util::FileNotFoundError;
use ragnarok_bytes::ConversionError;

#[derive(Debug)]
pub enum LoadError {
    File(FileNotFoundError),
    Conversion(Box<ConversionError>),
    UnsupportedFormat(String),
}
