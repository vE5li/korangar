use ragnarok_bytes::{ByteReader, ConversionResult, FromBytes, ToBytes};

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct WorldPosition {
    pub x: usize,
    pub y: usize,
}

impl WorldPosition {
    pub fn new(x: usize, y: usize) -> Self {
        Self { x, y }
    }
}

impl FromBytes for WorldPosition {
    fn from_bytes<Meta>(byte_reader: &mut ByteReader<Meta>) -> ConversionResult<Self> {
        let coordinates: Vec<usize> = byte_reader.slice::<Self>(3)?.iter().map(|byte| *byte as usize).collect();

        let x = (coordinates[1] >> 6) | (coordinates[0] << 2);
        let y = (coordinates[2] >> 4) | ((coordinates[1] & 0b111111) << 4);
        //let direction = ...

        Ok(Self { x, y })
    }
}

impl ToBytes for WorldPosition {
    fn to_bytes(&self) -> ConversionResult<Vec<u8>> {
        let mut coordinates = vec![0, 0, 0];

        coordinates[0] = (self.x >> 2) as u8;
        coordinates[1] = ((self.x << 6) as u8) | (((self.y >> 4) & 0x3F) as u8);
        coordinates[2] = (self.y << 4) as u8;

        Ok(coordinates)
    }
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct WorldPosition2 {
    pub x1: usize,
    pub y1: usize,
    pub x2: usize,
    pub y2: usize,
}

impl WorldPosition2 {
    pub fn new(x1: usize, y1: usize, x2: usize, y2: usize) -> Self {
        Self { x1, y1, x2, y2 }
    }

    pub fn to_origin_destination(self) -> (WorldPosition, WorldPosition) {
        (WorldPosition { x: self.x1, y: self.y1 }, WorldPosition {
            x: self.x2,
            y: self.y2,
        })
    }
}

impl FromBytes for WorldPosition2 {
    fn from_bytes<Meta>(byte_reader: &mut ByteReader<Meta>) -> ConversionResult<Self> {
        let coordinates: Vec<usize> = byte_reader.slice::<Self>(6)?.iter().map(|byte| *byte as usize).collect();

        let x1 = (coordinates[1] >> 6) | (coordinates[0] << 2);
        let y1 = (coordinates[2] >> 4) | ((coordinates[1] & 0b111111) << 4);
        let x2 = (coordinates[3] >> 2) | ((coordinates[2] & 0b1111) << 6);
        let y2 = coordinates[4] | ((coordinates[3] & 0b11) << 8);
        //let direction = ...

        Ok(Self { x1, y1, x2, y2 })
    }
}

impl ToBytes for WorldPosition2 {
    fn to_bytes(&self) -> ConversionResult<Vec<u8>> {
        let mut bytes = vec![0; 6];

        bytes[0] = (self.x1 >> 2) as u8;
        bytes[1] = ((self.x1 << 6) as u8) | ((self.y1 >> 4) as u8);
        bytes[2] = ((self.y1 << 4) as u8) | ((self.x2 >> 6) as u8);
        bytes[3] = ((self.x2 << 2) as u8) | ((self.y2 >> 8) as u8);
        bytes[4] = self.y2 as u8;

        Ok(bytes)
    }
}

#[cfg(test)]
mod conversion {
    use ragnarok_bytes::{FromBytes, ToBytes};

    use crate::{WorldPosition, WorldPosition2};

    #[test]
    fn world_position() {
        // Since we don't save the orientation when deserializing, this is a lossy
        // operation. So we construct some test cases that igonore the bits in
        // question.
        let cases = [[255, 0, 0], [0, 255, 0], [0, 0, 240]];

        for case in cases {
            let mut byte_reader = ragnarok_bytes::ByteReader::without_metadata(&case);

            let position = WorldPosition::from_bytes(&mut byte_reader).unwrap();
            let output = position.to_bytes().unwrap();

            assert_eq!(case.as_slice(), output.as_slice());
        }
    }

    #[test]
    fn world_position_2() {
        // Since we don't save the orientation when deserializing, this is a lossy
        // operation. So we construct some test cases that igonore the bits in
        // question.
        let cases = [
            [255, 0, 0, 0, 0, 0],
            [0, 255, 0, 0, 0, 0],
            [0, 0, 255, 0, 0, 0],
            [0, 0, 0, 255, 0, 0],
            [0, 0, 0, 0, 255, 0],
        ];

        for case in cases {
            let mut byte_reader = ragnarok_bytes::ByteReader::without_metadata(&case);

            let position = WorldPosition2::from_bytes(&mut byte_reader).unwrap();
            let output = position.to_bytes().unwrap();

            assert_eq!(case.as_slice(), output.as_slice());
        }
    }
}
