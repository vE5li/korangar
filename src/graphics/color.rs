#[derive(Copy, Clone, Debug)]
pub struct Color {
    pub red: u8,
    pub blue: u8,
    pub green: u8,
}

impl Color {

    pub const WHITE: Color = Color::new(255, 255, 255);

    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        return Self { red, green, blue };
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
}
