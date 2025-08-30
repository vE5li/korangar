#[derive(Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum HorizontalAlignment {
    Left { offset: f32, border: f32 },
    Center { offset: f32, border: f32 },
    Right { offset: f32, border: f32 },
}

impl HorizontalAlignment {
    pub fn scaled(&self, scaling: f32) -> Self {
        match self {
            Self::Left { offset, border } => Self::Left {
                offset: *offset * scaling,
                border: *border * scaling,
            },
            Self::Center { offset, border } => Self::Center {
                offset: *offset * scaling,
                border: *border * scaling,
            },
            Self::Right { offset, border } => Self::Right {
                offset: *offset * scaling,
                border: *border * scaling,
            },
        }
    }
}

#[derive(Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
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
