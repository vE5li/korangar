//! IVF errors.

use std::error::Error;

/// Errors that can occur when parsing IVF containers.
#[derive(Debug)]
pub enum IvfError {
    /// A std::io::Error.
    IoError(std::io::Error),
    /// A `TryFromSliceError`.
    TryFromSliceError(std::array::TryFromSliceError),
    /// A `TryFromIntError`.
    TryFromIntError(std::num::TryFromIntError),
    /// Invalid header.
    InvalidHeader(String),
    /// Unexpected file ending.
    UnexpectedFileEnding,
}

impl std::fmt::Display for IvfError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            IvfError::IoError(err) => {
                write!(f, "{:?}", err.source())
            }
            IvfError::TryFromSliceError(err) => {
                write!(f, "{:?}", err.source())
            }
            IvfError::TryFromIntError(err) => {
                write!(f, "{:?}", err.source())
            }
            IvfError::InvalidHeader(message) => {
                write!(f, "invalid header: {}", message)
            }
            IvfError::UnexpectedFileEnding => {
                write!(f, "unexpected file ending")
            }
        }
    }
}

impl From<std::io::Error> for IvfError {
    fn from(err: std::io::Error) -> IvfError {
        IvfError::IoError(err)
    }
}

impl From<std::array::TryFromSliceError> for IvfError {
    fn from(err: std::array::TryFromSliceError) -> IvfError {
        IvfError::TryFromSliceError(err)
    }
}

impl From<std::num::TryFromIntError> for IvfError {
    fn from(err: std::num::TryFromIntError) -> IvfError {
        IvfError::TryFromIntError(err)
    }
}

impl Error for IvfError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match *self {
            IvfError::IoError(ref e) => Some(e),
            IvfError::TryFromSliceError(ref e) => Some(e),
            IvfError::TryFromIntError(ref e) => Some(e),
            _ => None,
        }
    }
}
