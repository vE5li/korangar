use procedural::*;
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Color {
    pub red: u8,
    pub blue: u8,
    pub green: u8,
    pub alpha: u8,
}

impl Color {
    pub const fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha: 255,
        }
    }

    pub fn rgb_f32(red: f32, green: f32, blue: f32) -> Self {
        let red = (red * 255.0) as u8;
        let green = (green * 255.0) as u8;
        let blue = (blue * 255.0) as u8;

        Self {
            red,
            green,
            blue,
            alpha: 255,
        }
    }

    pub fn rgb_hex(hex: &str) -> Self {
        assert_eq!(hex.len(), 6);

        let channel = |range| u8::from_str_radix(&hex[range], 16).unwrap();
        Color::rgb(channel(0..2), channel(2..4), channel(4..6))
    }

    pub const fn rgba(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self { red, green, blue, alpha }
    }

    pub const fn monochrome(brightness: u8) -> Self {
        Self {
            red: brightness,
            green: brightness,
            blue: brightness,
            alpha: 255,
        }
    }

    pub fn red_f32(&self) -> f32 {
        self.red as f32 / 255.0
    }

    pub fn green_f32(&self) -> f32 {
        self.green as f32 / 255.0
    }

    pub fn blue_f32(&self) -> f32 {
        self.blue as f32 / 255.0
    }

    pub fn alpha_f32(&self) -> f32 {
        self.alpha as f32 / 255.0
    }

    pub fn invert(&self) -> Self {
        Self::rgba(255 - self.red, 255 - self.green, 255 - self.green, self.alpha)
    }

    pub fn shade(&self) -> Self {
        match (self.red as usize) + (self.green as usize) + (self.blue as usize) > 382 {
            true => Self::rgba(
                self.red.saturating_sub(40),
                self.green.saturating_sub(40),
                self.blue.saturating_sub(40),
                self.alpha,
            ),
            false => Self::rgba(
                self.red.saturating_add(40),
                self.green.saturating_add(40),
                self.blue.saturating_add(40),
                self.alpha,
            ),
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug, ByteConvertable, PrototypeElement)]
pub struct ColorBGRA {
    pub blue: u8,
    pub green: u8,
    pub red: u8,
    pub alpha: u8,
}

impl From<ColorBGRA> for Color {
    fn from(color: ColorBGRA) -> Self {
        Self::rgba(color.red, color.green, color.blue, color.alpha)
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug, ByteConvertable, PrototypeElement)]
pub struct ColorRGB {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
}

impl From<ColorRGB> for Color {
    fn from(color: ColorRGB) -> Self {
        Self::rgb_f32(color.red, color.green, color.blue)
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug, ByteConvertable, PrototypeElement)]
pub struct ColorRGBA {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

impl From<ColorRGBA> for Color {
    fn from(color: ColorRGBA) -> Self {
        Self::rgba(color.red, color.green, color.blue, color.alpha)
    }
}
