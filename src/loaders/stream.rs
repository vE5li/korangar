use std::cell::RefCell;
use std::rc::Rc;

use cgmath::{Matrix3, Vector3};
use derive_new::new;

#[cfg(feature = "debug")]
use crate::debug::*;
use crate::graphics::Color;
#[cfg(feature = "debug_network")]
use crate::interface::PacketEntry;
#[cfg(feature = "debug_network")]
use crate::interface::TrackedState;
use crate::loaders::Version;
#[cfg(feature = "debug")]
use crate::network::Packet;

#[derive(new)]
pub struct ByteStream<'b> {
    data: &'b [u8],
    #[new(default)]
    offset: usize,
    #[new(default)]
    version: Option<Version>,
    #[cfg(feature = "debug_network")]
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

    pub fn set_version(&mut self, version: Version) {
        self.version = version.into();
    }

    pub fn get_version(&mut self) -> Version {
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

    pub fn version(&mut self) -> Version {
        let major = self.next();
        let minor = self.next();

        Version::new(major, minor)
    }

    pub fn byte(&mut self) -> u8 {
        self.next()
    }

    pub fn integer16(&mut self) -> i16 {
        let mut value = 0;

        value |= self.next() as i16;
        value |= (self.next() as i16) << 8;

        value
    }

    pub fn integer32(&mut self) -> i32 {
        let mut value = 0;

        value |= self.next() as i32;
        value |= (self.next() as i32) << 8;
        value |= (self.next() as i32) << 16;
        value |= (self.next() as i32) << 24;

        value
    }

    pub fn string(&mut self, count: usize) -> String {
        let mut value = String::new();

        for index in 0..count {
            let byte = self.next();

            if byte == 0 {
                self.skip(count - index - 1);
                break;
            }

            value.push(byte as char);
        }

        value
    }

    pub fn float32(&mut self) -> f32 {
        let first = self.next();
        let second = self.next();
        let third = self.next();
        let fourth = self.next();

        f32::from_le_bytes([first, second, third, fourth])
    }

    pub fn vector3(&mut self) -> Vector3<f32> {
        let x = self.float32();
        let y = self.float32();
        let z = self.float32();

        Vector3::new(x, y, z)
    }

    pub fn vector3_flipped(&mut self) -> Vector3<f32> {
        let x = self.float32();
        let y = self.float32();
        let z = self.float32();

        Vector3::new(x, -y, z)
    }

    pub fn color(&mut self) -> Color {
        let red = self.float32();
        let green = self.float32();
        let blue = self.float32();

        Color::rgb((red * 255.0) as u8, (green * 255.0) as u8, (blue * 255.0) as u8)
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

    #[cfg(feature = "debug_network")]
    pub fn incoming_packet(&mut self, packet: &(impl Packet + 'static), name: &'static str, is_ping: bool) {
        self.packet_history.push(PacketEntry::new_incoming(packet, name, is_ping));
    }

    #[cfg(feature = "debug_network")]
    pub fn incoming_unknown_packet(&mut self, bytes: Vec<u8>) {
        self.packet_history.push(PacketEntry::new_incoming(&bytes, "UNKNOWN", false));
    }

    #[cfg(feature = "debug_network")]
    pub fn transfer_packet_history(&mut self, packet_history: &mut TrackedState<Vec<PacketEntry>>) {
        packet_history.append(&mut self.packet_history);
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
