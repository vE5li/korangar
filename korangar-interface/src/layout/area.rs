use super::HoverCheck;
use crate::prelude::{HorizontalAlignment, VerticalAlignment};

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

    pub fn interior(
        self,
        width: f32,
        height: f32,
        horizontal_alignment: HorizontalAlignment,
        vertical_alignment: VerticalAlignment,
    ) -> Self {
        let left = self.left
            + match horizontal_alignment {
                HorizontalAlignment::Left { offset, .. } => offset,
                HorizontalAlignment::Center { offset, .. } => (self.width - width) / 2.0 + offset,
                HorizontalAlignment::Right { offset, .. } => self.width - width - offset,
            };

        let top = self.top
            + match vertical_alignment {
                VerticalAlignment::Top { offset } => offset,
                VerticalAlignment::Center { offset } => (self.height - height) / 2.0 + offset,
                VerticalAlignment::Bottom { offset } => self.height - height - offset,
            };

        Self { left, top, width, height }
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
