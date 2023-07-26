use derive_new::new;

use super::version::InternalVersion;
use super::Version;
#[cfg(feature = "debug")]
use crate::debug::*;
#[cfg(feature = "debug")]
use crate::interface::PacketEntry;
#[cfg(feature = "debug")]
use crate::interface::TrackedState;
#[cfg(feature = "debug")]
use crate::network::Packet;

#[derive(new)]
pub struct ByteStream<'b> {
    data: &'b [u8],
    #[new(default)]
    offset: usize,
    #[new(default)]
    version: Option<InternalVersion>,
    #[cfg(feature = "debug")]
    #[new(default)]
    packet_history: Vec<PacketEntry>,
}

impl<'b> ByteStream<'b> {
    pub fn next(&mut self) -> u8 {
        assert!(self.offset < self.data.len(), "byte stream is shorter than expected");
        let byte = self.data[self.offset];
        self.offset += 1;
        byte
    }

    pub fn peek(&self, index: usize) -> u8 {
        assert!(self.offset + index < self.data.len(), "byte stream is shorter than expected");
        self.data[self.offset + index]
    }

    pub fn is_empty(&self) -> bool {
        self.offset >= self.data.len()
    }

    pub fn set_version<T>(&mut self, version: Version<T>) {
        self.version = Some(version.into());
    }

    pub fn get_version(&mut self) -> InternalVersion {
        self.version.unwrap()
    }

    pub fn match_signature(&mut self, signature: [u8; 2]) -> bool {
        if self.data.len() - self.offset < 2 {
            return false;
        }

        let signature_matches = self.data[self.offset] == signature[0] && self.data[self.offset + 1] == signature[1];

        if signature_matches {
            self.offset += 2;
        }

        signature_matches
    }

    pub fn slice(&mut self, count: usize) -> Vec<u8> {
        let mut value = Vec::new();

        for _index in 0..count {
            let byte = self.next();
            value.push(byte);
        }

        value
    }

    pub fn remaining_bytes(&mut self) -> Vec<u8> {
        // temporary ?
        self.slice(self.data.len() - self.offset)
    }

    pub fn skip(&mut self, count: usize) {
        self.offset += count;
    }

    pub fn get_offset(&self) -> usize {
        self.offset
    }

    #[cfg(feature = "debug")]
    pub fn incoming_packet(&mut self, packet: &(impl Packet + 'static), name: &'static str, is_ping: bool) {
        self.packet_history.push(PacketEntry::new_incoming(packet, name, is_ping));
    }

    #[cfg(feature = "debug")]
    pub fn incoming_unknown_packet(&mut self, bytes: Vec<u8>) {
        self.packet_history.push(PacketEntry::new_incoming(&bytes, "UNKNOWN", false));
    }

    #[cfg(feature = "debug")]
    pub fn transfer_packet_history<const N: usize>(&mut self, packet_history: &mut TrackedState<RingBuffer<PacketEntry, N>>) {
        if !self.packet_history.is_empty() {
            packet_history.with_mut(|buffer, changed| {
                self.packet_history.drain(..).for_each(|packet| buffer.push(packet));
                changed()
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
