use std::any::TypeId;
#[cfg(feature = "debug")]
use std::cell::UnsafeCell;

use super::convertable::ConversionError;
use super::{ConversionErrorType, ConversionResult, Named};
#[cfg(feature = "debug")]
use crate::debug::*;
#[cfg(feature = "debug")]
use crate::interface::PacketEntry;
#[cfg(feature = "debug")]
use crate::interface::TrackedState;
use crate::interface::{ValueState, WeakElementCell};
use crate::loaders::convertable::check_upper_bound;
#[cfg(feature = "debug")]
use crate::network::IncomingPacket;

pub struct ByteStream<'a, META = ()>
where
    META: 'static,
{
    data: &'a [u8],
    offset: usize,
    metadata: META,
    #[cfg(feature = "debug")]
    packet_history: Vec<PacketEntry>,
}

impl<'a, META> ByteStream<'a, META>
where
    META: Default + 'static,
{
    pub fn without_metadata(data: &'a [u8]) -> Self {
        Self {
            data,
            offset: 0,
            metadata: META::default(),
            #[cfg(feature = "debug")]
            packet_history: Vec::new(),
        }
    }
}

impl<'a, META> ByteStream<'a, META>
where
    META: 'static,
{
    pub fn with_metadata(data: &'a [u8], metadata: META) -> Self {
        Self {
            data,
            offset: 0,
            metadata,
            #[cfg(feature = "debug")]
            packet_history: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.offset >= self.data.len()
    }

    pub fn get_metadata<CALLER, OUTER>(&self) -> ConversionResult<&OUTER>
    where
        OUTER: 'static,
        CALLER: Named,
    {
        match TypeId::of::<META>() == TypeId::of::<OUTER>() {
            true => unsafe { Ok(std::mem::transmute::<_, &OUTER>(&self.metadata)) },
            false => Err(ConversionError::from_error_type(ConversionErrorType::IncorrectMetadata {
                type_name: CALLER::NAME,
            })),
        }
    }

    pub fn get_metadata_mut<CALLER, OUTER>(&mut self) -> ConversionResult<&mut OUTER>
    where
        OUTER: 'static,
        CALLER: Named,
    {
        match TypeId::of::<META>() == TypeId::of::<OUTER>() {
            true => unsafe { Ok(std::mem::transmute::<_, &mut OUTER>(&mut self.metadata)) },
            false => Err(ConversionError::from_error_type(ConversionErrorType::IncorrectMetadata {
                type_name: CALLER::NAME,
            })),
        }
    }

    pub fn get_offset(&self) -> usize {
        self.offset
    }

    pub fn set_offset(&mut self, offset: usize) {
        self.offset = offset
    }

    pub fn next<S: Named>(&mut self) -> ConversionResult<u8> {
        check_upper_bound::<S>(self.offset, self.data.len())?;
        let byte = self.data[self.offset];
        self.offset += 1;
        Ok(byte)
    }

    pub fn slice<S: Named>(&mut self, count: usize) -> ConversionResult<&[u8]> {
        check_upper_bound::<S>(self.offset + count, self.data.len() + 1)?;

        let start_index = self.offset;
        self.offset += count;

        Ok(&self.data[start_index..self.offset])
    }

    pub fn remaining_bytes(&mut self) -> Vec<u8> {
        let end_index = self.data.len();
        let data = self.data[self.offset..end_index].to_vec();
        self.offset = end_index;
        data
    }

    pub fn into_metadata(self) -> META {
        self.metadata
    }

    #[cfg(feature = "debug")]
    pub fn incoming_packet<T: IncomingPacket + Clone + 'static>(&mut self, packet: &T) {
        self.packet_history.push(PacketEntry::new_incoming(packet, T::NAME, T::IS_PING));
    }

    #[cfg(feature = "debug")]
    pub fn transfer_packet_history<const N: usize>(
        &mut self,
        packet_history: &mut TrackedState<RingBuffer<(PacketEntry, UnsafeCell<Option<WeakElementCell>>), N>>,
    ) {
        if !self.packet_history.is_empty() {
            packet_history.with_mut(|buffer| {
                self.packet_history
                    .drain(..)
                    .for_each(|packet| buffer.push((packet, UnsafeCell::new(None))));
                ValueState::Mutated(())
            });
        }
    }

    #[cfg(feature = "debug")]
    pub fn assert_empty(&self, file_name: &str) {
        let remaining = self.data.len() - self.offset;

        if remaining != 0 {
            print_debug!(
                "incomplete read on file {}{}{}; {}{}{} bytes remaining",
                MAGENTA,
                file_name,
                NONE,
                YELLOW,
                remaining,
                NONE
            );
        }
    }
}
