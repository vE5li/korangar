#[derive(Debug, Clone, Copy)]
pub enum MessageColor {
    Rgb { red: u8, green: u8, blue: u8 },
    Broadcast,
    Server,
    Error,
    Information,
}
