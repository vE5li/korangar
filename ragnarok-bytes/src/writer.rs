use encoding_rs::{EUC_KR, Encoding};

use crate::{ConversionError, ConversionErrorType, ConversionResult};

/// A writer of bytes into a [`Vec<u8>`].
///
/// used in conjunction with the [`ToBytes`] trait.
pub struct ByteWriter {
    data: Vec<u8>,
    encoding: &'static Encoding,
}

impl Default for ByteWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl ByteWriter {
    /// Creates a new [`ByteWriter`]. The default encoding is `EUC_KR`.
    pub fn new() -> Self {
        Self {
            data: Vec::default(),
            encoding: EUC_KR,
        }
    }

    /// Creates a new [`ByteWriter`] that uses the given encoding to encode
    /// strings.
    pub fn with_encoding(encoding: &'static Encoding) -> Self {
        Self {
            data: Vec::default(),
            encoding,
        }
    }

    /// Executes the given write function and returns the count of bytes
    /// written.
    pub fn write_counted(&mut self, write: impl FnOnce(&mut Self) -> ConversionResult<()>) -> ConversionResult<usize> {
        let start = self.data.len();
        write(self)?;
        Ok(self.data.len() - start)
    }

    /// Encodes the given string and appends it's bytes to the data.
    pub fn encode_string(&mut self, string: &str) {
        let (bytes, ..) = self.encoding.encode(string);
        self.data.extend_from_slice(bytes.as_ref());
        self.data.push(0);
    }

    /// Adds the given byte to the data.
    #[inline(always)]
    pub fn push(&mut self, byte: u8) {
        self.data.push(byte);
    }

    /// Pops a single byte.
    #[inline(always)]
    pub fn pop(&mut self) {
        self.data.pop();
    }

    /// Adds the given bytes to the data.
    #[inline(always)]
    pub fn extend_from_slice(&mut self, bytes: &[u8]) {
        self.data.extend_from_slice(bytes);
    }

    /// Adds the given count of bytes to the data.
    #[inline(always)]
    pub fn extend(&mut self, add_count: usize, value: u8) {
        self.data.extend(std::iter::repeat_n(value, add_count));
    }

    /// Returns the current length of the data.
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns `true` if there is no inner data.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Overwrites bytes at the given position with the provided slice
    #[inline(always)]
    pub fn overwrite_at<const SIZE: usize>(&mut self, position: usize, bytes: [u8; SIZE]) -> ConversionResult<()> {
        let end = position + bytes.len();
        if end > self.data.len() {
            return Err(ConversionError::from_error_type(ConversionErrorType::DataTooBig {
                type_name: std::any::type_name::<[u8; SIZE]>(),
            }));
        }
        self.data[position..end].copy_from_slice(&bytes);
        Ok(())
    }

    /// Returns the inner bytes.
    #[must_use]
    pub fn into_inner(self) -> Vec<u8> {
        self.data
    }

    /// Returns a slice to the inner bytes.
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        self.data.as_slice()
    }

    /// Clears the inner bytes.
    pub fn clear(&mut self) {
        self.data.clear();
    }
}
