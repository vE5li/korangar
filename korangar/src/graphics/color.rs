use std::ops::{Add, Mul, Sub};

use ragnarok_formats::color::{ColorBGRA, ColorRGB};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Color {
    pub red: f32,
    pub blue: f32,
    pub green: f32,
    pub alpha: f32,
}

impl Color {
    pub const BLACK: Self = Self::monochrome(0.0);
    pub const WHITE: Self = Self::monochrome(1.0);

    pub const fn rgb(red: f32, green: f32, blue: f32) -> Self {
        Self {
            red,
            green,
            blue,
            alpha: 1.0,
        }
    }

    pub const fn rgba(red: f32, green: f32, blue: f32, alpha: f32) -> Self {
        Self { red, green, blue, alpha }
    }

    pub fn rgb_u8(red: u8, green: u8, blue: u8) -> Self {
        let red = (red as f32) / 255.0;
        let green = (green as f32) / 255.0;
        let blue = (blue as f32) / 255.0;

        Self {
            red,
            green,
            blue,
            alpha: 1.0,
        }
    }

    pub fn rgba_u8(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        let red = (red as f32) / 255.0;
        let green = (green as f32) / 255.0;
        let blue = (blue as f32) / 255.0;
        let alpha = (alpha as f32) / 255.0;

        Self { red, green, blue, alpha }
    }

    pub fn rgb_hex(hex: &str) -> Self {
        assert_eq!(hex.len(), 6);

        let channel = |range| u8::from_str_radix(&hex[range], 16).unwrap();
        Color::rgb_u8(channel(0..2), channel(2..4), channel(4..6))
    }

    pub const fn monochrome(brightness: f32) -> Self {
        Self {
            red: brightness,
            green: brightness,
            blue: brightness,
            alpha: 1.0,
        }
    }

    pub fn monochrome_u8(brightness: u8) -> Self {
        let brightness = (brightness as f32) / 255.0;
        Self {
            red: brightness,
            green: brightness,
            blue: brightness,
            alpha: 1.0,
        }
    }

    pub fn red_as_u8(&self) -> u8 {
        (self.red * 255.0) as u8
    }

    pub fn green_as_u8(&self) -> u8 {
        (self.green * 255.0) as u8
    }

    pub fn blue_as_u8(&self) -> u8 {
        (self.blue * 255.0) as u8
    }

    pub fn alpha_as_u8(&self) -> u8 {
        (self.alpha * 255.0) as u8
    }

    #[cfg(feature = "debug")]
    pub fn multiply_alpha(mut self, alpha: f32) -> Self {
        self.alpha *= alpha;
        self
    }

    pub fn invert(&self) -> Self {
        Self::rgba(1.0 - self.red, 1.0 - self.blue, 1.0 - self.green, self.alpha)
    }

    pub fn shade(&self) -> Self {
        match (self.red_as_u8() as usize) + (self.green_as_u8() as usize) + (self.blue_as_u8() as usize) > 382 {
            true => Self::rgba_u8(
                self.red_as_u8().saturating_sub(40),
                self.green_as_u8().saturating_sub(40),
                self.blue_as_u8().saturating_sub(40),
                self.alpha_as_u8(),
            ),
            false => Self::rgba_u8(
                self.red_as_u8().saturating_add(40),
                self.green_as_u8().saturating_add(40),
                self.blue_as_u8().saturating_add(40),
                self.alpha_as_u8(),
            ),
        }
    }

    pub fn components_linear(self) -> [f32; 4] {
        let srgb = [self.red, self.green, self.blue];
        let linear = srgb.map(|channel| {
            if channel <= 0.04045 {
                channel / 12.92
            } else {
                ((channel + 0.055) / 1.055).powf(2.4)
            }
        });
        [linear[0], linear[1], linear[2], self.alpha]
    }
}

impl Add for Color {
    type Output = Color;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            red: self.red + rhs.red,
            blue: self.blue + rhs.blue,
            green: self.green + rhs.green,
            alpha: self.alpha + rhs.alpha,
        }
    }
}

impl Sub for Color {
    type Output = Color;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            red: self.red - rhs.red,
            blue: self.blue - rhs.blue,
            green: self.green - rhs.green,
            alpha: self.alpha - rhs.alpha,
        }
    }
}

impl Mul<f32> for Color {
    type Output = Color;

    fn mul(self, rhs: f32) -> Self::Output {
        Self {
            red: self.red * rhs,
            blue: self.blue * rhs,
            green: self.green * rhs,
            alpha: self.alpha * rhs,
        }
    }
}

impl From<Color> for [f32; 3] {
    fn from(val: Color) -> Self {
        [val.red, val.green, val.blue]
    }
}

impl From<Color> for [f32; 4] {
    fn from(val: Color) -> Self {
        [val.red, val.green, val.blue, val.alpha]
    }
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

impl From<ColorBGRA> for Color {
    fn from(value: ColorBGRA) -> Self {
        let ColorBGRA { red, blue, green, alpha } = value;
        Color::rgba_u8(red, green, blue, alpha)
    }
}
