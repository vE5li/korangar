use std::any::TypeId;

use encoding_rs::{Encoding, EUC_KR};

use crate::{ConversionError, ConversionErrorType, ConversionResult};

/// Saved state of a [`ByteReader`] that can be restored.
#[derive(Debug, PartialEq, Eq)]
pub struct SavePoint {
    offset: usize,
    limit: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct TemporaryLimit {
    frame_limit: usize,
    old_limit: usize,
}

/// A reader of bytes that iterates over borrowed data. It can produce single
/// bytes or slices of memory and carries metadata about the read operation (for
/// example a version).
///
/// The reader is intended for reading data without lookahead.
/// The reader reads strings with the default encoding of "EUC-KR", which can be
/// changed by calling [`set_encoding`](ByteReader::set_encoding).
///
/// The state of the reader can be saved at any time with
/// [`create_save_point`](ByteReader::create_save_point), and restored with
/// [`restore_save_point`](ByteReader::restore_save_point).
///
/// NOTE: The save point *does not* restore the previous state of the metadata.
/// It should therefore be avoided to modify the metadata while reading of
/// composite structures data that might fail, for example multi-field structs
/// that implement [`FromBytes`](crate::from_bytes::FromBytes).
pub struct ByteReader<'a, Meta = ()>
where
    Meta: 'static,
{
    data: &'a [u8],
    encoding: &'static Encoding,
    offset: usize,
    limit: usize,
    metadata: Meta,
}

impl<'a> ByteReader<'a, ()> {
    /// Create a new [`ByteReader`] without metadata.
    pub fn without_metadata(data: &'a [u8]) -> Self {
        Self::with_metadata(data, ())
    }
}

impl<'a, Meta> ByteReader<'a, Meta>
where
    Meta: Default + 'static,
{
    /// Create a new [`ByteReader`] with default metadata.
    pub fn with_default_metadata(data: &'a [u8]) -> Self {
        Self::with_metadata(data, Default::default())
    }
}

impl<'a, Meta> ByteReader<'a, Meta>
where
    Meta: 'static,
{
    /// Create a new [`ByteReader`] with specific metadata.
    pub fn with_metadata(data: &'a [u8], metadata: Meta) -> Self {
        let limit = data.len();

        Self {
            data,
            encoding: EUC_KR,
            offset: 0,
            limit,
            metadata,
        }
    }

    /// Sets the encoding used to decode strings.
    pub fn set_encoding(&mut self, encoding: &'static Encoding) {
        self.encoding = encoding;
    }

    pub fn decode_string(&mut self, bytes: &[u8]) -> String {
        let (cow, error) = self.encoding.decode_without_bom_handling(bytes);
        if error {
            bytes.iter().map(|byte| *byte as char).collect()
        } else {
            cow.to_string()
        }
    }

    pub fn get_offset(&self) -> usize {
        self.offset
    }

    // TODO: Implement this only for readers with metadata that can not be mutated
    // while reading.
    //
    // E.g: `Reusable` or `Rollback` trait.
    pub fn create_save_point(&self) -> SavePoint {
        SavePoint {
            offset: self.offset,
            limit: self.limit,
        }
    }

    // TODO: Implement this only for readers with metadata that can not be mutated
    // while reading.
    //
    // E.g: `Reusable` or `Rollback` trait.
    pub fn restore_save_point(&mut self, save_point: SavePoint) {
        self.offset = save_point.offset;
        self.limit = save_point.limit;
    }

    pub(crate) fn install_limit<Caller>(&mut self, size: usize) -> ConversionResult<TemporaryLimit> {
        let frame_limit = self.offset + size;
        let old_limit = self.limit;

        if frame_limit > old_limit {
            return Err(ConversionError::from_error_type(ConversionErrorType::ByteReaderTooShort {
                type_name: std::any::type_name::<Caller>(),
            }));
        }

        self.limit = frame_limit;

        Ok(TemporaryLimit { frame_limit, old_limit })
    }

    pub(crate) fn uninstall_limit(&mut self, limits: TemporaryLimit) {
        self.offset = limits.frame_limit;
        self.limit = limits.old_limit;
    }

    pub fn is_empty(&self) -> bool {
        self.offset >= self.limit
    }

    pub fn get_metadata<Caller, As>(&self) -> ConversionResult<&As>
    where
        As: 'static,
    {
        match TypeId::of::<Meta>() == TypeId::of::<As>() {
            true => unsafe { Ok(std::mem::transmute::<_, &As>(&self.metadata)) },
            false => Err(ConversionError::from_error_type(ConversionErrorType::IncorrectMetadata {
                type_name: std::any::type_name::<Caller>(),
            })),
        }
    }

    pub fn get_metadata_mut<Caller, As>(&mut self) -> ConversionResult<&mut As>
    where
        As: 'static,
    {
        match TypeId::of::<Meta>() == TypeId::of::<As>() {
            true => unsafe { Ok(std::mem::transmute::<_, &mut As>(&mut self.metadata)) },
            false => Err(ConversionError::from_error_type(ConversionErrorType::IncorrectMetadata {
                type_name: std::any::type_name::<Caller>(),
            })),
        }
    }

    pub fn into_metadata(self) -> Meta {
        self.metadata
    }

    fn check_upper_bound<Caller>(offset: usize, length: usize) -> ConversionResult<()> {
        match offset < length {
            true => Ok(()),
            false => Err(ConversionError::from_error_type(ConversionErrorType::ByteReaderTooShort {
                type_name: std::any::type_name::<Caller>(),
            })),
        }
    }

    #[inline(always)]
    fn byte_unchecked(&mut self) -> u8 {
        let byte = self.data[self.offset];
        self.offset += 1;
        byte
    }

    pub fn byte<Caller>(&mut self) -> ConversionResult<u8> {
        Self::check_upper_bound::<Caller>(self.offset, self.limit)?;
        Ok(self.byte_unchecked())
    }

    pub fn bytes<Caller, const LENGTH: usize>(&mut self) -> ConversionResult<[u8; LENGTH]> {
        Self::check_upper_bound::<Caller>(self.offset + LENGTH.saturating_sub(1), self.limit)?;
        Ok(std::array::from_fn(|_| self.byte_unchecked()))
    }

    pub fn slice<Caller>(&mut self, count: usize) -> ConversionResult<&[u8]> {
        Self::check_upper_bound::<Caller>(self.offset + count, self.limit + 1)?;

        let start_index = self.offset;
        self.offset += count;

        Ok(&self.data[start_index..self.offset])
    }

    pub fn remaining_bytes(&mut self) -> Vec<u8> {
        let data = self.data[self.offset..self.limit].to_vec();
        self.offset = self.limit;
        data
    }
}

#[cfg(test)]
mod save_point {
    use crate::ByteReader;

    const TEST_BYTE_SIZE: usize = 10;

    #[test]
    fn restore() {
        let mut byte_reader = ByteReader::without_metadata(&[0; TEST_BYTE_SIZE]);

        let save_point = byte_reader.create_save_point();

        assert_eq!(byte_reader.offset, 0);
        assert_eq!(byte_reader.limit, TEST_BYTE_SIZE);

        byte_reader.offset = TEST_BYTE_SIZE / 2;
        byte_reader.limit = TEST_BYTE_SIZE / 2;

        assert_eq!(byte_reader.offset, TEST_BYTE_SIZE / 2);
        assert_eq!(byte_reader.limit, TEST_BYTE_SIZE / 2);

        byte_reader.restore_save_point(save_point);

        assert_eq!(byte_reader.offset, 0);
        assert_eq!(byte_reader.limit, TEST_BYTE_SIZE);
    }
}

#[cfg(test)]
mod temporary_limit {
    use crate::reader::TemporaryLimit;
    use crate::ByteReader;

    const TEST_BASE_OFFSET: usize = 1;
    const TEST_BYTE_SIZE: usize = 10;

    #[test]
    fn install() {
        let mut byte_reader = ByteReader::without_metadata(&[0; TEST_BYTE_SIZE]);
        byte_reader.offset = TEST_BASE_OFFSET;
        let result = byte_reader.install_limit::<()>(TEST_BYTE_SIZE / 2);

        assert_eq!(byte_reader.limit, TEST_BASE_OFFSET + TEST_BYTE_SIZE / 2);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), TemporaryLimit {
            frame_limit: TEST_BASE_OFFSET + TEST_BYTE_SIZE / 2,
            old_limit: TEST_BYTE_SIZE
        });
    }

    #[test]
    fn install_too_big() {
        let mut byte_reader = ByteReader::without_metadata(&[0; TEST_BYTE_SIZE]);
        byte_reader.offset = TEST_BASE_OFFSET;
        let result = byte_reader.install_limit::<()>(TEST_BYTE_SIZE * 2);

        assert!(result.is_err());
    }

    #[test]
    fn uninstall() {
        let mut byte_reader = ByteReader::without_metadata(&[0; TEST_BYTE_SIZE]);
        let temporary_limit = byte_reader.install_limit::<()>(TEST_BYTE_SIZE / 2).unwrap();
        byte_reader.uninstall_limit(temporary_limit);

        assert_eq!(byte_reader.limit, TEST_BYTE_SIZE);
        assert_eq!(byte_reader.offset, TEST_BYTE_SIZE / 2);
    }
}

#[cfg(test)]
mod metadata {
    use crate::ByteReader;

    #[test]
    fn get_metadata() {
        let byte_reader = ByteReader::<i32>::with_metadata(&[0; 1], 9);

        assert!(byte_reader.get_metadata::<(), i32>().is_ok());
        assert!(byte_reader.get_metadata::<(), u32>().is_err());
    }

    #[test]
    fn get_metadata_mut() {
        let mut byte_reader = ByteReader::<i32>::with_metadata(&[0; 1], 9);

        assert!(byte_reader.get_metadata_mut::<(), i32>().is_ok());
        assert!(byte_reader.get_metadata_mut::<(), u32>().is_err());
    }

    #[test]
    fn into_metadata() {
        let byte_reader = ByteReader::<i32>::with_metadata(&[0; 1], 9);

        assert_eq!(byte_reader.into_metadata(), 9);
    }
}

#[cfg(test)]
mod byte {
    use std::assert_matches::assert_matches;

    use crate::ByteReader;

    #[test]
    fn under_limit() {
        let mut byte_reader = ByteReader::without_metadata(&[9; 1]);

        assert_matches!(byte_reader.byte::<()>(), Ok(9));
    }

    #[test]
    fn over_limit() {
        let mut byte_reader = ByteReader::without_metadata(&[9; 1]);

        assert!(byte_reader.byte::<()>().is_ok());
        assert!(byte_reader.byte::<()>().is_err());
    }
}

#[cfg(test)]
mod bytes {
    use std::assert_matches::assert_matches;

    use crate::ByteReader;

    #[test]
    fn under_limit() {
        let mut byte_reader = ByteReader::without_metadata(&[9; 4]);

        assert_matches!(byte_reader.bytes::<(), 4>(), Ok([9, 9, 9, 9]));
    }

    #[test]
    fn over_limit() {
        let mut byte_reader = ByteReader::without_metadata(&[9; 4]);

        assert!(byte_reader.bytes::<(), 5>().is_err());
    }
}

#[cfg(test)]
mod slice {
    use std::assert_matches::assert_matches;

    use crate::ByteReader;

    #[test]
    fn smaller_than_limit() {
        let mut byte_reader = ByteReader::without_metadata(&[9; 4]);

        assert_matches!(byte_reader.slice::<()>(3), Ok(&[9, 9, 9]));
        assert_eq!(byte_reader.remaining_bytes().as_slice(), &[9]);
    }

    #[test]
    fn exactly_on_limit() {
        let mut byte_reader = ByteReader::without_metadata(&[9; 4]);

        assert_matches!(byte_reader.slice::<()>(4), Ok(&[9, 9, 9, 9]));
        assert!(byte_reader.is_empty());
    }

    #[test]
    fn bigger_than_limit() {
        let mut byte_reader = ByteReader::without_metadata(&[9; 4]);
        let result = byte_reader.slice::<()>(5);

        assert!(result.is_err());
    }
}

#[cfg(test)]
mod remaining_bytes {
    use crate::ByteReader;

    const TEST_BYTES: &[u8] = &[1, 2, 3];

    #[test]
    fn some_remaining() {
        let mut byte_reader = ByteReader::without_metadata(TEST_BYTES);

        assert_eq!(byte_reader.remaining_bytes().as_slice(), TEST_BYTES);
    }

    #[test]
    fn none_remaining() {
        let mut byte_reader = ByteReader::without_metadata(TEST_BYTES);

        assert!(byte_reader.slice::<()>(TEST_BYTES.len()).is_ok());
        assert!(byte_reader.remaining_bytes().is_empty());
    }
}
