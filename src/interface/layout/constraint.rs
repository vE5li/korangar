use derive_new::new;
use procedural::constraint;
use serde::{Deserialize, Serialize};

use crate::interface::{Dimension, PartialSize, Position, Size};

#[derive(Copy, Clone, Serialize, Deserialize, new)]
pub struct DimensionConstraint {
    pub size: Dimension,
    pub minimum_size: Option<Dimension>,
    pub maximum_size: Option<Dimension>,
}

impl DimensionConstraint {
    pub fn add_height(&self, height_constraint: DimensionConstraint) -> SizeConstraint {
        SizeConstraint {
            width: self.size,
            minimum_width: self.minimum_size,
            maximum_width: self.maximum_size,
            height: height_constraint.size,
            minimum_height: height_constraint.minimum_size,
            maximum_height: height_constraint.maximum_size,
        }
    }
}

#[derive(Copy, Clone, Serialize, Deserialize, new)]
pub struct SizeConstraint {
    pub width: Dimension,
    pub minimum_width: Option<Dimension>,
    pub maximum_width: Option<Dimension>,
    pub height: Dimension,
    pub minimum_height: Option<Dimension>,
    pub maximum_height: Option<Dimension>,
}

impl SizeConstraint {
    pub fn resolve(&self, available: Size, remaining: Size, scaling: f32) -> PartialSize {
        let width = self.width.resolve_width(available.x, remaining.x, scaling);
        let width = self.validated_width(width, available.x, remaining.x, scaling);

        let mut height = self.height.resolve_height(available.y.into(), remaining.y.into(), scaling);
        if let Some(height) = &mut height {
            *height = self.validated_height(*height, available.y.into(), remaining.y.into(), scaling);
        }

        PartialSize::new(width, height)
    }

    pub fn resolve_partial(&self, available: PartialSize, remaining: PartialSize, scaling: f32) -> PartialSize {
        let width = self.width.resolve_width(available.x, remaining.x, scaling);
        let width = self.validated_width(width, available.x, remaining.x, scaling);

        let mut height = self.height.resolve_height(available.y, remaining.y, scaling);
        if let Some(height) = &mut height {
            *height = self.validated_height(*height, available.y, remaining.y, scaling);
        }

        PartialSize::new(width, height)
    }

    fn validated_width(&self, mut width: f32, available: f32, remaining: f32, scaling: f32) -> f32 {
        if let Some(maximum_width) = self.maximum_width {
            let maximum_value = maximum_width.resolve_width(available, remaining, scaling);
            width = f32::min(width, maximum_value);
        }

        if let Some(minimum_width) = self.minimum_width {
            let minimum_value = minimum_width.resolve_width(available, remaining, scaling);
            width = f32::max(width, minimum_value);
        }

        width
    }

    pub fn validated_height(&self, mut height: f32, available: Option<f32>, remaining: Option<f32>, scaling: f32) -> f32 {
        if let Some(maximum_height) = self.maximum_height {
            let maximum_value = maximum_height.resolve_height(available, remaining, scaling);
            height = f32::min(height, maximum_value.expect("maximum height cannot be flexible"));
        }

        if let Some(minimum_height) = self.minimum_height {
            let minimum_value = minimum_height.resolve_height(available, remaining, scaling);
            height = f32::max(height, minimum_value.expect("minimum height cannot be flexible"));
        }

        height
    }

    pub fn validated_size(&self, size: Size, available: Size, scaling: f32) -> Size {
        let width = self.validated_width(size.x, available.x, available.x, scaling);
        let height = self.validated_height(size.y, available.y.into(), available.y.into(), scaling);
        Size::new(width, height)
    }

    pub fn validated_position(&self, position: Position, size: Size, available: Size) -> Position {
        let half_size = size / 2.0;
        let x = f32::clamp(position.x, -half_size.x, available.x - half_size.x);
        let y = f32::clamp(position.y, 0.0, available.y - 30.0);
        Position::new(x, y)
    }
}

impl Default for SizeConstraint {
    fn default() -> Self {
        constraint!(200 > 300 < 400, 100 > ? < 80%)
    }
}
