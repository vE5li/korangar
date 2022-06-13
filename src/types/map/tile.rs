use derive_new::new;

const NONE: u8 = 0b00000000;
const WALKABLE: u8 = 0b00000001;
const WATER: u8 = 0b00000010;
const SNIPABLE: u8 = 0b00000100;
const CLIFF: u8 = 0b00001000;

#[derive(Copy, Clone, Debug)]
pub struct TileType(u8);

impl TileType {

    pub fn new(type_index: u8) -> Self {
        match type_index {
            0 => return Self(WALKABLE),
            1 => return Self(NONE),
            2 => return Self(WATER),
            3 => return Self(WATER | WALKABLE),
            4 => return Self(WATER | SNIPABLE),
            5 => return Self(CLIFF | SNIPABLE),
            6 => return Self(CLIFF),
            invalid => panic!("invalid tile type {}", invalid),
        }
    }

    pub fn is_none(&self) -> bool {
        return self.0 == 0;
    }

    pub fn is_walkable(&self) -> bool {
        return self.0 & WALKABLE != 0;
    }
}

#[derive(Clone, new)]
pub struct Tile {
    pub upper_left_height: f32,
    pub upper_right_height: f32,
    pub lower_left_height: f32,
    pub lower_right_height: f32,
    pub tile_type: TileType,
}

impl Tile {

    pub fn is_walkable(&self) -> bool {
        return self.tile_type.is_walkable();
    }
}
