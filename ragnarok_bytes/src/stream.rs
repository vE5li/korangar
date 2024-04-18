use std::any::TypeId;

use crate::{ConversionError, ConversionErrorType, ConversionResult};

pub struct SavePoint {
    offset: usize,
    limit: usize,
}

pub(crate) struct TemporaryLimit {
    frame_limit: usize,
    old_limit: usize,
}

pub struct ByteStream<'a, META = ()>
where
    META: 'static,
{
    data: &'a [u8],
    offset: usize,
    limit: usize,
    metadata: META,
}

impl<'a, META> ByteStream<'a, META>
where
    META: Default + 'static,
{
    pub fn without_metadata(data: &'a [u8]) -> Self {
        Self::with_metadata(data, Default::default())
    }
}

impl<'a, META> ByteStream<'a, META>
where
    META: 'static,
{
    pub fn with_metadata(data: &'a [u8], metadata: META) -> Self {
        let limit = data.len();

        Self {
            data,
            offset: 0,
            limit,
            metadata,
        }
    }

    pub fn get_offset(&self) -> usize {
        self.offset
    }

    // TODO: Implement this only for streams with metadata that can not be mutated
    // while reading.
    //
    // E.g: `Reusable` or `Rollback` trait.
    pub fn create_save_point(&self) -> SavePoint {
        SavePoint {
            offset: self.offset,
            limit: self.limit,
        }
    }

    // TODO: Implement this only for streams with metadata that can not be mutated
    // while reading.
    //
    // E.g: `Reusable` or `Rollback` trait.
    pub fn restore_save_point(&mut self, save_point: SavePoint) {
        self.offset = save_point.offset;
        self.limit = save_point.limit;
    }

    pub(crate) fn install_limit<CALLER>(&mut self, size: usize) -> ConversionResult<TemporaryLimit> {
        let frame_limit = self.offset + size;
        let old_limit = self.limit;

        if frame_limit > old_limit {
            return Err(ConversionError::from_error_type(ConversionErrorType::ByteStreamTooShort {
                type_name: std::any::type_name::<CALLER>(),
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

    pub fn get_metadata<CALLER, OUTER>(&self) -> ConversionResult<&OUTER>
    where
        OUTER: 'static,
    {
        match TypeId::of::<META>() == TypeId::of::<OUTER>() {
            true => unsafe { Ok(std::mem::transmute::<_, &OUTER>(&self.metadata)) },
            false => Err(ConversionError::from_error_type(ConversionErrorType::IncorrectMetadata {
                type_name: std::any::type_name::<CALLER>(),
            })),
        }
    }

    pub fn get_metadata_mut<CALLER, OUTER>(&mut self) -> ConversionResult<&mut OUTER>
    where
        OUTER: 'static,
    {
        match TypeId::of::<META>() == TypeId::of::<OUTER>() {
            true => unsafe { Ok(std::mem::transmute::<_, &mut OUTER>(&mut self.metadata)) },
            false => Err(ConversionError::from_error_type(ConversionErrorType::IncorrectMetadata {
                type_name: std::any::type_name::<CALLER>(),
            })),
        }
    }

    pub fn into_metadata(self) -> META {
        self.metadata
    }

    fn check_upper_bound<CALLER>(offset: usize, length: usize) -> ConversionResult<()> {
        match offset < length {
            true => Ok(()),
            false => Err(ConversionError::from_error_type(ConversionErrorType::ByteStreamTooShort {
                type_name: std::any::type_name::<CALLER>(),
            })),
        }
    }

    pub fn byte<CALLER>(&mut self) -> ConversionResult<u8> {
        Self::check_upper_bound::<CALLER>(self.offset, self.limit)?;

        let byte = self.data[self.offset];
        self.offset += 1;
        Ok(byte)
    }

    pub fn slice<CALLER>(&mut self, count: usize) -> ConversionResult<&[u8]> {
        Self::check_upper_bound::<CALLER>(self.offset + count, self.limit + 1)?;

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
