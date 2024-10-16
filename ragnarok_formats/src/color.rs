use ragnarok_bytes::ByteConvertable;

#[derive(Debug, Clone, Copy, PartialEq, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct ColorRGB {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
}

impl ColorRGB {
    pub fn clamp_color_channels(&mut self) {
        self.red = self.red.clamp(0.0, 1.0);
        self.green = self.green.clamp(0.0, 1.0);
        self.blue = self.blue.clamp(0.0, 1.0);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct ColorBGRA {
    pub blue: u8,
    pub green: u8,
    pub red: u8,
    pub alpha: u8,
}
