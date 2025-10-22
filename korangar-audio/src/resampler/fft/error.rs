use std::{error, fmt};

/// The error type used by the resampler.
pub(crate) enum ResampleError {
    /// Error raised when the number of frames in an input buffer is less
    /// than the minimum expected.
    InsufficientInputBufferSize { expected: usize, actual: usize },
    /// Error raised when the number of frames in an output buffer is less
    /// than the minimum expected.
    InsufficientOutputBufferSize { expected: usize, actual: usize },
}

impl fmt::Display for ResampleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InsufficientInputBufferSize { expected, actual } => {
                write!(f, "Insufficient input buffer size {actual}, expected {expected} frames")
            }
            Self::InsufficientOutputBufferSize { expected, actual } => {
                write!(f, "Insufficient output buffer size {actual}, expected {expected} frames")
            }
        }
    }
}

impl fmt::Debug for ResampleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self}")
    }
}

impl error::Error for ResampleError {}
