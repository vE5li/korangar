#[derive(Clone, Copy)]
pub enum HorizontalAlignment {
    Left { offset: f32 },
    Center { offset: f32 },
    Right { offset: f32 },
}

impl HorizontalAlignment {
    pub fn scaled(&self, scaling: f32) -> Self {
        match self {
            Self::Left { offset } => Self::Left { offset: *offset * scaling },
            Self::Center { offset } => Self::Center { offset: *offset * scaling },
            Self::Right { offset } => Self::Right { offset: *offset * scaling },
        }
    }
}

#[derive(Clone, Copy)]
pub enum VerticalAlignment {
    Top { offset: f32 },
    Center { offset: f32 },
    Bottom { offset: f32 },
}

impl VerticalAlignment {
    pub fn scaled(&self, scaling: f32) -> Self {
        match self {
            Self::Top { offset } => Self::Top { offset: *offset * scaling },
            Self::Center { offset } => Self::Center { offset: *offset * scaling },
            Self::Bottom { offset } => Self::Bottom { offset: *offset * scaling },
        }
    }
}
