use korangar_interface::elements::PrototypeElement;
use ragnarok_bytes::ByteConvertable;

use crate::graphics::Color;

#[derive(Clone, Debug, ByteConvertable, PrototypeElement)]
pub struct ColorRGB {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
}

impl From<ColorRGB> for Color {
    fn from(value: ColorRGB) -> Self {
        let ColorRGB { red, blue, green } = value;
        Color {
            red,
            green,
            blue,
            alpha: 1.0,
        }
    }
}
