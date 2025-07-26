#[derive(Clone, Copy)]
pub enum HorizontalAlignment {
    Left { offset: f32 },
    Center { offset: f32 },
    Right { offset: f32 },
}

#[derive(Clone, Copy)]
pub enum VerticalAlignment {
    Top { offset: f32 },
    Center { offset: f32 },
    Bottom { offset: f32 },
}
