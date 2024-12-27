use ragnarok_bytes::{ByteConvertable, ByteReader, ConversionResult, FromBytes, ToBytes};

#[derive(Debug, Copy, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub enum Direction {
    N = 0,
    NE = 1,
    E = 2,
    SE = 3,
    S = 4,
    SW = 5,
    W = 6,
    NW = 7,
}

impl From<Direction> for usize {
    fn from(value: Direction) -> Self {
        value as usize
    }
}

impl From<usize> for Direction {
    fn from(value: usize) -> Self {
        let value = value & 7;

        match value {
            0 => Direction::N,
            1 => Direction::NE,
            2 => Direction::E,
            3 => Direction::SE,
            4 => Direction::S,
            5 => Direction::SW,
            6 => Direction::W,
            7 => Direction::NW,
            _ => unreachable!(),
        }
    }
}

impl From<[isize; 2]> for Direction {
    fn from(value: [isize; 2]) -> Self {
        match value {
            [0, 1] => Direction::N,
            [1, 1] => Direction::NE,
            [1, 0] => Direction::E,
            [1, -1] => Direction::SE,
            [0, -1] => Direction::S,
            [-1, -1] => Direction::SW,
            [-1, 0] => Direction::W,
            [-1, 1] => Direction::NW,
            _ => panic!("impossible direction"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct WorldPosition {
    pub x: usize,
    pub y: usize,
    pub direction: Direction,
}

impl WorldPosition {
    pub fn new(x: usize, y: usize, direction: Direction) -> Self {
        Self { x, y, direction }
    }

    pub fn origin() -> Self {
        Self {
            x: 0,
            y: 0,
            direction: Direction::N,
        }
    }
}

impl FromBytes for WorldPosition {
    fn from_bytes<Meta>(byte_reader: &mut ByteReader<Meta>) -> ConversionResult<Self> {
        let coordinates: Vec<usize> = byte_reader.slice::<Self>(3)?.iter().map(|byte| *byte as usize).collect();

        let x = (coordinates[1] >> 6) | (coordinates[0] << 2);
        let y = (coordinates[2] >> 4) | ((coordinates[1] & 0b111111) << 4);
        let mut direction = coordinates[2] & 0b1111;
        direction = (8 - direction + 4) & 7;

        Ok(Self {
            x,
            y,
            direction: direction.into(),
        })
    }
}

impl ToBytes for WorldPosition {
    fn to_bytes(&self) -> ConversionResult<Vec<u8>> {
        let mut coordinates = vec![0, 0, 0];
        let direction = (8 - usize::from(self.direction) + 4) & 7;

        coordinates[0] = (self.x >> 2) as u8;
        coordinates[1] = ((self.x << 6) as u8) | (((self.y >> 4) & 0x3F) as u8);
        coordinates[2] = (self.y << 4) as u8 | (direction & 0xF) as u8;

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
    pub unknown: usize,
}

impl WorldPosition2 {
    pub fn new(x1: usize, y1: usize, x2: usize, y2: usize) -> Self {
        Self {
            x1,
            y1,
            x2,
            y2,
            unknown: 0,
        }
    }

    pub fn to_origin_destination(self) -> (WorldPosition, WorldPosition) {
        (
            WorldPosition {
                x: self.x1,
                y: self.y1,
                direction: Direction::N,
            },
            WorldPosition {
                x: self.x2,
                y: self.y2,
                direction: Direction::N,
            },
        )
    }
}

impl FromBytes for WorldPosition2 {
    fn from_bytes<Meta>(byte_reader: &mut ByteReader<Meta>) -> ConversionResult<Self> {
        let coordinates: Vec<usize> = byte_reader.slice::<Self>(6)?.iter().map(|byte| *byte as usize).collect();

        let x1 = (coordinates[1] >> 6) | (coordinates[0] << 2);
        let y1 = (coordinates[2] >> 4) | ((coordinates[1] & 0b111111) << 4);
        let x2 = (coordinates[3] >> 2) | ((coordinates[2] & 0b1111) << 6);
        let y2 = coordinates[4] | ((coordinates[3] & 0b11) << 8);
        let unknown = coordinates[5];

        Ok(Self { x1, y1, x2, y2, unknown })
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
        bytes[5] = self.unknown as u8;

        Ok(bytes)
    }
}

#[cfg(test)]
mod conversion {
    use ragnarok_bytes::{FromBytes, ToBytes};

    use crate::{WorldPosition, WorldPosition2};

    #[test]
    fn world_position() {
        // The direction must be between 0 and 7 inclusive.
        let direction = [0, 3, 7];
        let cases = [[255, 0, 0 + direction[0]], [0, 255, 0 + direction[1]], [0, 0, 240 + direction[2]]];

        for case in cases {
            let mut byte_reader = ragnarok_bytes::ByteReader::without_metadata(&case);

            let position = WorldPosition::from_bytes(&mut byte_reader).unwrap();
            let output = position.to_bytes().unwrap();

            assert_eq!(case.as_slice(), output.as_slice());
        }
    }

    #[test]
    fn world_position_2() {
        let cases = [
            [255, 0, 0, 0, 0, 0],
            [0, 255, 0, 0, 0, 0],
            [0, 0, 255, 0, 0, 0],
            [0, 0, 0, 255, 0, 0],
            [0, 0, 0, 0, 255, 0],
            [0, 0, 0, 0, 0, 255],
        ];

        for case in cases {
            let mut byte_reader = ragnarok_bytes::ByteReader::without_metadata(&case);

            let position = WorldPosition2::from_bytes(&mut byte_reader).unwrap();
            let output = position.to_bytes().unwrap();

            assert_eq!(case.as_slice(), output.as_slice());
        }
    }
}
