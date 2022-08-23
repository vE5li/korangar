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
            0 => ShadingType::None,
            1 => ShadingType::FlatShading,
            2 => ShadingType::SmoothShading,
            3 => ShadingType::Black,
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
        write!(formatter, "{}", self.display_name())
    }
}
