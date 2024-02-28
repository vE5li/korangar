use derive_new::new;
use procedural::constraint;
use serde::{Deserialize, Serialize};

use crate::interface::{Dimension, PartialScreenSize, ScreenPosition, ScreenSize};

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
    pub fn resolve(&self, available: ScreenSize, remaining: ScreenSize, scaling: f32) -> PartialScreenSize {
        let width = self.width.resolve_width(available.width, remaining.width, scaling);
        let width = self.validated_width(width, available.width, remaining.width, scaling);

        let mut height = self
            .height
            .resolve_height(available.height.into(), remaining.height.into(), scaling);
        if let Some(height) = &mut height {
            *height = self.validated_height(*height, available.height.into(), remaining.height.into(), scaling);
        }

        PartialScreenSize::new(width, height)
    }

    pub fn resolve_partial(&self, available: PartialScreenSize, remaining: PartialScreenSize, scaling: f32) -> PartialScreenSize {
        let width = self.width.resolve_width(available.width, remaining.width, scaling);
        let width = self.validated_width(width, available.width, remaining.width, scaling);

        let mut height = self.height.resolve_height(available.height, remaining.height, scaling);
        if let Some(height) = &mut height {
            *height = self.validated_height(*height, available.height, remaining.height, scaling);
        }

        PartialScreenSize::new(width, height)
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

    pub fn validated_size(&self, size: ScreenSize, available: ScreenSize, scaling: f32) -> ScreenSize {
        let width = self.validated_width(size.width, available.width, available.width, scaling);
        let height = self.validated_height(size.height, available.height.into(), available.height.into(), scaling);

        ScreenSize { width, height }
    }

    pub fn validated_position(&self, position: ScreenPosition, size: ScreenSize, available: ScreenSize) -> ScreenPosition {
        let half_size = size / 2.0;
        let left = f32::clamp(position.left, -half_size.width, available.width - half_size.width);
        let top = f32::clamp(position.top, 0.0, available.height - 30.0);

        ScreenPosition { left, top }
    }
}

impl Default for SizeConstraint {
    fn default() -> Self {
        constraint!(200 > 300 < 400, 100 > ? < 80%)
    }
}
