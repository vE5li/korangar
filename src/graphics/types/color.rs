use serde::{ Serialize, Deserialize };

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub struct Color {
    pub red: u8,
    pub blue: u8,
    pub green: u8,
}

impl Color {

    pub const fn rgb(red: u8, green: u8, blue: u8) -> Self {
        return Self { red, green, blue };
    }

    pub const fn monochrome(brightness: u8) -> Self {
        return Self { red: brightness, green: brightness, blue: brightness };
    }

    pub fn red_f32(&self) -> f32 {
        return self.red as f32 / 255.0;
    }

    pub fn green_f32(&self) -> f32 {
        return self.green as f32 / 255.0;
    }

    pub fn blue_f32(&self) -> f32 {
        return self.blue as f32 / 255.0;
    }

    pub fn invert(&self) -> Self {
        Self::rgb(255 - self.red, 255 - self.green, 255 - self.green)
    }

    pub fn shade(&self) -> Self {
        match (self.red as usize) + (self.green as usize) + (self.blue as usize) > 382 {
            true => Self::rgb(self.red.saturating_sub(40), self.green.saturating_sub(40), self.blue.saturating_sub(40)),
            false => Self::rgb(self.red.saturating_add(40), self.green.saturating_add(40), self.blue.saturating_add(40)),
        }
    }
}
