use ragnarok_bytes::{ByteConvertable, ByteReader, ByteWriter, ConversionResult, FromBytes, ToBytes};

use crate::TilePosition;

#[derive(Debug, Copy, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub enum Direction {
    North = 0,
    NorthEast = 1,
    East = 2,
    SouthEast = 3,
    South = 4,
    SouthWest = 5,
    West = 6,
    NorthWest = 7,
}

impl From<Direction> for u16 {
    fn from(value: Direction) -> Self {
        value as u16
    }
}

impl From<u16> for Direction {
    fn from(value: u16) -> Self {
        let value = value & 7;

        match value {
            0 => Direction::North,
            1 => Direction::NorthEast,
            2 => Direction::East,
            3 => Direction::SouthEast,
            4 => Direction::South,
            5 => Direction::SouthWest,
            6 => Direction::West,
            7 => Direction::NorthWest,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct InvalidDirectionError;

impl TryFrom<[isize; 2]> for Direction {
    type Error = InvalidDirectionError;

    fn try_from(value: [isize; 2]) -> Result<Self, Self::Error> {
        match value {
            [0, 1] => Ok(Direction::North),
            [1, 1] => Ok(Direction::NorthEast),
            [1, 0] => Ok(Direction::East),
            [1, -1] => Ok(Direction::SouthEast),
            [0, -1] => Ok(Direction::South),
            [-1, -1] => Ok(Direction::SouthWest),
            [-1, 0] => Ok(Direction::West),
            [-1, 1] => Ok(Direction::NorthWest),
            _ => Err(InvalidDirectionError),
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct WorldPosition {
    pub x: u16,
    pub y: u16,
    pub direction: Direction,
}

impl WorldPosition {
    pub fn new(x: u16, y: u16, direction: Direction) -> Self {
        Self { x, y, direction }
    }

    pub fn origin() -> Self {
        Self {
            x: 0,
            y: 0,
            direction: Direction::North,
        }
    }

    pub fn tile_position(&self) -> TilePosition {
        TilePosition { x: self.x, y: self.y }
    }
}

impl FromBytes for WorldPosition {
    fn from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self> {
        let coordinates: Vec<u16> = byte_reader.slice::<Self>(3)?.iter().map(|byte| *byte as u16).collect();

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
    fn to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        byte_writer.write_counted(|write| {
            let mut coordinates = [0, 0, 0];
            let direction = (8 - u16::from(self.direction) + 4) & 7;

            coordinates[0] = (self.x >> 2) as u8;
            coordinates[1] = ((self.x << 6) as u8) | (((self.y >> 4) & 0x3F) as u8);
            coordinates[2] = (self.y << 4) as u8 | (direction & 0xF) as u8;

            write.extend_from_slice(&coordinates);

            Ok(())
        })
    }
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct WorldPosition2 {
    pub x1: u16,
    pub y1: u16,
    pub x2: u16,
    pub y2: u16,
    pub unknown: u16,
}

impl WorldPosition2 {
    pub fn new(x1: u16, y1: u16, x2: u16, y2: u16) -> Self {
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
                direction: Direction::North,
            },
            WorldPosition {
                x: self.x2,
                y: self.y2,
                direction: Direction::North,
            },
        )
    }
}

impl FromBytes for WorldPosition2 {
    fn from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self> {
        let coordinates: Vec<u16> = byte_reader.slice::<Self>(6)?.iter().map(|byte| *byte as u16).collect();

        let x1 = (coordinates[1] >> 6) | (coordinates[0] << 2);
        let y1 = (coordinates[2] >> 4) | ((coordinates[1] & 0b111111) << 4);
        let x2 = (coordinates[3] >> 2) | ((coordinates[2] & 0b1111) << 6);
        let y2 = coordinates[4] | ((coordinates[3] & 0b11) << 8);
        let unknown = coordinates[5];

        Ok(Self { x1, y1, x2, y2, unknown })
    }
}

impl ToBytes for WorldPosition2 {
    fn to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        byte_writer.write_counted(|write| {
            let mut bytes = [0; 6];

            bytes[0] = (self.x1 >> 2) as u8;
            bytes[1] = ((self.x1 << 6) as u8) | ((self.y1 >> 4) as u8);
            bytes[2] = ((self.y1 << 4) as u8) | ((self.x2 >> 6) as u8);
            bytes[3] = ((self.x2 << 2) as u8) | ((self.y2 >> 8) as u8);
            bytes[4] = self.y2 as u8;
            bytes[5] = self.unknown as u8;

            write.extend_from_slice(&bytes);

            Ok(())
        })
    }
}

#[cfg(test)]
mod conversion {
    use ragnarok_bytes::{ByteWriter, FromBytes, ToBytes};

    use crate::{WorldPosition, WorldPosition2};

    #[test]
    fn world_position() {
        // The direction must be between 0 and 7 inclusive.
        let direction = [0, 3, 7];
        let cases = [[255, 0, direction[0]], [0, 255, direction[1]], [0, 0, 240 + direction[2]]];

        for case in cases {
            let mut byte_reader = ragnarok_bytes::ByteReader::without_metadata(&case);

            let position = WorldPosition::from_bytes(&mut byte_reader).unwrap();

            let mut byte_writer = ByteWriter::new();
            position.to_bytes(&mut byte_writer).unwrap();

            assert_eq!(case.as_slice(), byte_writer.into_inner().as_slice());
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

            let mut byte_writer = ByteWriter::new();
            position.to_bytes(&mut byte_writer).unwrap();

            assert_eq!(case.as_slice(), byte_writer.into_inner().as_slice());
        }
    }
}
