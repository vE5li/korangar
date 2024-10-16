use super::HoverCheck;

#[derive(Debug, Clone, Copy)]
pub struct Area {
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
}

impl Area {
    pub fn check(self) -> HoverCheck {
        HoverCheck::new(self)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PartialArea {
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: Option<f32>,
}

impl From<Area> for PartialArea {
    fn from(Area { left, top, width, height }: Area) -> Self {
        Self {
            left,
            top,
            width,
            height: Some(height),
        }
    }
}
