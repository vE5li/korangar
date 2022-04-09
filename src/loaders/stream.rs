use derive_new::new;
use std::slice::Iter;

#[cfg(feature = "debug")]
use debug::*;
use maths::*;
use graphics::Color;

use super::Version;

#[derive(new)]
pub struct ByteStream<'b> {
    iterator: Iter<'b, u8>,
    #[cfg(feature = "debug")]
    #[new(default)]
    counter: usize,
}

impl<'b> ByteStream<'b> {

    fn next(&mut self) -> u8 {

        #[cfg(feature = "debug")]
        { self.counter += 1; }

        return *self.iterator.next().unwrap();
    }

    pub fn version(&mut self) -> Version {

        let major = self.next();
        let minor = self.next();

        return Version::new(major, minor);
    }

    pub fn byte(&mut self) -> u8 {
        return self.next();
    }

    pub fn integer16(&mut self) -> i16 {
        let mut value = 0;

        value |= self.next() as i16;
        value |= (self.next() as i16) << 8;

        return value;
    }

    pub fn integer32(&mut self) -> i32 {
        let mut value = 0;

        value |= self.next() as i32;
        value |= (self.next() as i32) << 8;
        value |= (self.next() as i32) << 16;
        value |= (self.next() as i32) << 24;

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

    pub fn vector3_flipped(&mut self) -> Vector3<f32> {

        let x = self.float32();
        let y = self.float32();
        let z = self.float32();

        return Vector3::new(x, -y, z);
    }

    pub fn matrix3(&mut self) -> Matrix3<f32> {

        let c0r0 = self.float32();
        let c0r1 = self.float32();
        let c0r2 = self.float32();

        let c1r0 = self.float32();
        let c1r1 = self.float32();
        let c1r2 = self.float32();

        let c2r0 = self.float32();
        let c2r1 = self.float32();
        let c2r2 = self.float32();

        return Matrix3::new(c0r0, c0r1, c0r2, c1r0, c1r1, c1r2, c2r0, c2r1, c2r2);
    }

    pub fn color(&mut self) -> Color {

        let red = self.float32();
        let green = self.float32();
        let blue = self.float32();

        return Color::new((red * 255.0) as u8, (green * 255.0) as u8, (blue * 255.0) as u8);
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

    #[cfg(feature = "debug")]
    pub fn assert_empty(&self, length: usize, file_name: &str) {
        let remaining = length - self.counter;

        if remaining != 0 {
            print_debug!("incomplete read on file {}{}{}; {}{}{} bytes remaining", magenta(), file_name, none(), yellow(), remaining, none());
        }
    }
}
