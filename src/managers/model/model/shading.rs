use std::fmt::{ Display, Formatter, Result };

pub enum ShadingType {
    None,
    FlatShading,
    SmoothShading,
    Black,
}

impl ShadingType {

    pub fn from(raw: usize) -> Self {
        match raw {
            0 => return ShadingType::None,
            1 => return ShadingType::FlatShading,
            2 => return ShadingType::SmoothShading,
            3 => return ShadingType::Black,
            invalid => panic!("invalid shading type {}", invalid), // return result ?
        }
    }

    fn display_name(&self) -> &'static str {
        match self {
            ShadingType::None => "none",
            ShadingType::FlatShading => "flat shading",
            ShadingType::SmoothShading => "smooth shading",
            ShadingType::Black => "block",
        }
    }
}

impl Display for ShadingType {

    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result {
        return write!(formatter, "{}", self.display_name());
    }
}
