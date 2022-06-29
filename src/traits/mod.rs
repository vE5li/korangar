use types::ByteStream;

pub trait ByteConvertable {

    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self;

    fn to_bytes(&self, length_hint: Option<usize>) -> Vec<u8> {
        panic!()
    }
}

impl ByteConvertable for u8 {

    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
        assert!(length_hint.is_none(), "u8 may not have a length hint");
        byte_stream.next()
    }

    fn to_bytes(&self, length_hint: Option<usize>) -> Vec<u8> {
        assert!(length_hint.is_none(), "u8 may not have a length hint");
        vec![*self]
    }
}

impl ByteConvertable for u16 {

    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
        assert!(length_hint.is_none(), "u16 may not have a length hint");

        let mut value = 0;

        value |= byte_stream.next() as u16;
        value |= (byte_stream.next() as u16) << 8;

        value
    }

    fn to_bytes(&self, length_hint: Option<usize>) -> Vec<u8> {
        assert!(length_hint.is_none(), "u16 may not have a length hint");
        vec![*self as u8, (*self >> 8) as u8]
    }
}

impl ByteConvertable for u32 {

    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
        assert!(length_hint.is_none(), "u32 may not have a length hint");

        let mut value = 0;

        value |= byte_stream.next() as u32;
        value |= (byte_stream.next() as u32) << 8;
        value |= (byte_stream.next() as u32) << 16;
        value |= (byte_stream.next() as u32) << 24;

        value
    }

    fn to_bytes(&self, length_hint: Option<usize>) -> Vec<u8> {
        assert!(length_hint.is_none(), "u32 may not have a length hint");
        vec![*self as u8, (*self >> 8) as u8, (*self >> 16) as u8, (*self >> 24) as u8]
    }
}

impl ByteConvertable for u64 {

    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
        assert!(length_hint.is_none(), "u64 may not have a length hint");

        let mut value = 0;

        value |= byte_stream.next() as u64;
        value |= (byte_stream.next() as u64) << 8;
        value |= (byte_stream.next() as u64) << 16;
        value |= (byte_stream.next() as u64) << 24;
        value |= (byte_stream.next() as u64) << 32;
        value |= (byte_stream.next() as u64) << 40;
        value |= (byte_stream.next() as u64) << 48;
        value |= (byte_stream.next() as u64) << 56;

        value
    }

    fn to_bytes(&self, length_hint: Option<usize>) -> Vec<u8> {
        assert!(length_hint.is_none(), "u64 may not have a length hint");
        vec![*self as u8, (*self >> 8) as u8, (*self >> 16) as u8, (*self >> 24) as u8, (*self >> 32) as u8, (*self >> 40) as u8, (*self >> 48) as u8, (*self >> 56) as u8]
    }
}

impl ByteConvertable for i8 {

    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
        assert!(length_hint.is_none(), "i8 may not have a length hint");
        byte_stream.next() as i8
    }

    fn to_bytes(&self, length_hint: Option<usize>) -> Vec<u8> {
        assert!(length_hint.is_none(), "i8 may not have a length hint");
        vec![*self as u8]
    }
}

impl ByteConvertable for i16 {

    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
        assert!(length_hint.is_none(), "i16 may not have a length hint");

        let mut value = 0;

        value |= byte_stream.next() as i16;
        value |= (byte_stream.next() as i16) << 8;

        value
    }

    fn to_bytes(&self, length_hint: Option<usize>) -> Vec<u8> {
        assert!(length_hint.is_none(), "i16 may not have a length hint");
        vec![*self as u8, (*self >> 8) as u8]
    }
}

impl ByteConvertable for i32 {

    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
        assert!(length_hint.is_none(), "i32 may not have a length hint");

        let mut value = 0;

        value |= byte_stream.next() as i32;
        value |= (byte_stream.next() as i32) << 8;
        value |= (byte_stream.next() as i32) << 16;
        value |= (byte_stream.next() as i32) << 24;

        value
    }

    fn to_bytes(&self, length_hint: Option<usize>) -> Vec<u8> {
        assert!(length_hint.is_none(), "i32 may not have a length hint");
        vec![*self as u8, (*self >> 8) as u8, (*self >> 16) as u8, (*self >> 24) as u8]
    }
}

impl ByteConvertable for i64 {

    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
        assert!(length_hint.is_none(), "i64 may not have a length hint");

        let mut value = 0;

        value |= byte_stream.next() as i64;
        value |= (byte_stream.next() as i64) << 8;
        value |= (byte_stream.next() as i64) << 16;
        value |= (byte_stream.next() as i64) << 24;
        value |= (byte_stream.next() as i64) << 32;
        value |= (byte_stream.next() as i64) << 40;
        value |= (byte_stream.next() as i64) << 48;
        value |= (byte_stream.next() as i64) << 56;

        value
    }

    fn to_bytes(&self, length_hint: Option<usize>) -> Vec<u8> {
        assert!(length_hint.is_none(), "i64 may not have a length hint");
        vec![*self as u8, (*self >> 8) as u8, (*self >> 16) as u8, (*self >> 24) as u8, (*self >> 32) as u8, (*self >> 40) as u8, (*self >> 48) as u8, (*self >> 56) as u8]
    }
}

impl<T: Copy + Default + ByteConvertable, const SIZE: usize> ByteConvertable for [T; SIZE] {

    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
        assert!(length_hint.is_none(), "array may not have a length hint");
 
        let mut value = [T::default(); SIZE];

        for index in 0..SIZE {
            value[index] = T::from_bytes(byte_stream, None);
        }

        value
    }

    fn to_bytes(&self, length_hint: Option<usize>) -> Vec<u8> {
        assert!(length_hint.is_none(), "array may not have a length hint");

        self
            .iter()
            .fold(Vec::new(), |mut bytes, value| {
                bytes.extend(value.to_bytes(None));
                bytes
            })
    }
}

impl ByteConvertable for String {

    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {

        let mut value = String::new();
        let mut offset = 0;

        loop {
            offset += 1;

            match byte_stream.next() {
                0 => break,
                byte => value.push(byte as char)
            }
        }

        if let Some(length) = length_hint {
            byte_stream.skip(length - offset); 
            // maybe error if no zero byte was found
        }

        value
    }

    fn to_bytes(&self, length_hint: Option<usize>) -> Vec<u8> {
        use std::iter;
 
        match length_hint {

            Some(length) => {
                assert!(self.len() <= length, "string is to long for the byte stream");
                let padding = (0..length - self.len()).into_iter().map(|_| 0);
                self.bytes().chain(padding).collect()
            },

            None => self.bytes().chain(iter::once(0)).collect(), 
        }
    }
}
