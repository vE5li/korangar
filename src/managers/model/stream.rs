use std::slice::Iter;
use cgmath::Vector3;

use super::Version;

pub struct ByteStream<'b> {
    iterator: Iter<'b, u8>,
    counter: usize,
}

impl<'b> ByteStream<'b> {

    pub fn new(iterator: Iter<'b, u8>) -> Self {
        let counter = 0;

        return Self { iterator, counter };
    }

    fn next(&mut self) -> u8 {
        self.counter += 1;
        return *self.iterator.next().unwrap();
    }

    pub fn version(&mut self) -> Version {

        let major = self.next();
        let minor = self.next();

        return Version::new(major, minor);
    }

    pub fn integer(&mut self, count: usize) -> u64 {

        // assert count <= 4
        let mut value = 0;

        for index in 0..count {
            let byte = self.next();
            value |= (byte as u64) << (index * 8);
        }

        return value;
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

        return value;
    }

    pub fn float32(&mut self) -> f32 {

        let first = self.next();
        let second = self.next();
        let third = self.next();
        let fourth = self.next();

        return f32::from_le_bytes([first, second, third, fourth]);
    }

    pub fn vector3(&mut self) -> Vector3<f32> {

        let x = self.float32();
        let y = self.float32();
        let z = self.float32();

        return Vector3::new(x, y, z);
    }

    pub fn slice(&mut self, count: usize) -> Vec<u8> { // replace with matrix 4x4 ?
        let mut value = Vec::new();

        for _index in 0..count {
            let byte = self.next();
            value.push(byte);
        }

        return value;
    }

    pub fn skip(&mut self, count: usize) {
        for _index in 0..count {
            self.next();
        }
    }
}
