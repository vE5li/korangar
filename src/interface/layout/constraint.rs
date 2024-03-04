use derive_new::new;
use procedural::constraint;
use serde::{Deserialize, Serialize};

use crate::interface::{Dimension, PartialScreenSize, ScreenPosition, ScreenSize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, new)]
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, new)]
pub struct SizeConstraint {
    pub width: Dimension,
    pub minimum_width: Option<Dimension>,
    pub maximum_width: Option<Dimension>,
    pub height: Dimension,
    pub minimum_height: Option<Dimension>,
    pub maximum_height: Option<Dimension>,
}

impl SizeConstraint {
    pub const DEFAULT_FULLY_BOUNDED: Self = constraint!(200 > 300 < 400, 0 > ? < 80%);
    pub const DEFAULT_UNBOUNDED: Self = constraint!(200 > 300 < 400, ?);

    pub fn resolve_window(&self, available: ScreenSize, remaining: ScreenSize, scaling: f32) -> PartialScreenSize {
        let parent_limits = ParentLimits::default();

        let width = self.width.resolve_width(available.width, remaining.width, None, scaling);
        let width = self.validated_width(width, available.width, remaining.width, &parent_limits, scaling);

        let mut height = self
            .height
            .resolve_height(Some(available.height), Some(remaining.height), None, scaling);

        if let Some(height) = &mut height {
            *height = self.validated_height(
                *height,
                available.height.into(),
                remaining.height.into(),
                &parent_limits,
                scaling,
            );
        }

        PartialScreenSize::new(width, height)
    }

    pub fn resolve_element(
        &self,
        available: PartialScreenSize,
        remaining: PartialScreenSize,
        parent_limits: &ParentLimits,
        scaling: f32,
    ) -> PartialScreenSize {
        let width = self.width.resolve_width(available.width, remaining.width, None, scaling);
        let width = self.validated_width(width, available.width, remaining.width, parent_limits, scaling);

        let mut height = self.height.resolve_height(available.height, remaining.height, None, scaling);
        if let Some(height) = &mut height {
            *height = self.validated_height(*height, available.height, remaining.height, parent_limits, scaling);
        }

        PartialScreenSize::new(width, height)
    }

    fn validated_width(&self, mut width: f32, available: f32, remaining: f32, parent_limits: &ParentLimits, scaling: f32) -> f32 {
        if let Some(maximum_width) = self.maximum_width {
            let maximum_value = maximum_width.resolve_width(available, remaining, parent_limits.maximum_width, scaling);
            width = f32::min(width, maximum_value);
        }

        if let Some(minimum_width) = self.minimum_width {
            let minimum_value = minimum_width.resolve_width(available, remaining, parent_limits.minimum_width, scaling);
            width = f32::max(width, minimum_value);
        }

        width
    }

    pub fn validated_height(
        &self,
        mut height: f32,
        available: Option<f32>,
        remaining: Option<f32>,
        parent_limits: &ParentLimits,
        scaling: f32,
    ) -> f32 {
        if let Some(maximum_height) = self.maximum_height {
            let maximum_value = maximum_height.resolve_height(available, remaining, parent_limits.maximum_height, scaling);
            height = f32::min(height, maximum_value.expect("maximum height cannot be flexible"));
        }

        if let Some(minimum_height) = self.minimum_height {
            let minimum_value = minimum_height.resolve_height(available, remaining, parent_limits.minimum_height, scaling);
            height = f32::max(height, minimum_value.expect("minimum height cannot be flexible"));
        }

        height
    }

    pub fn validated_window_size(&self, size: ScreenSize, available: ScreenSize, scaling: f32) -> ScreenSize {
        let parent_limits = ParentLimits::default();

        let width = self.validated_width(size.width, available.width, available.width, &parent_limits, scaling);
        let height = self.validated_height(
            size.height,
            Some(available.height),
            Some(available.height),
            &parent_limits,
            scaling,
        );

        ScreenSize { width, height }
    }

    pub fn validated_position(&self, position: ScreenPosition, size: ScreenSize, available: ScreenSize) -> ScreenPosition {
        let half_size = size / 2.0;
        let left = f32::clamp(position.left, -half_size.width, available.width - half_size.width);
        let top = f32::clamp(position.top, 0.0, available.height - 30.0);

        ScreenPosition { left, top }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ParentLimits {
    pub minimum_width: Option<f32>,
    pub maximum_width: Option<f32>,
    pub minimum_height: Option<f32>,
    pub maximum_height: Option<f32>,
}

impl ParentLimits {
    pub fn from_constraints(size_constraint: &SizeConstraint, available_space: ScreenSize, scaling: f32) -> Self {
        Self {
            minimum_width: size_constraint
                .minimum_width
                .and_then(|dimension| dimension.try_resolve_width(available_space.width, available_space.width, None, scaling)),
            maximum_width: size_constraint
                .maximum_width
                .and_then(|dimension| dimension.try_resolve_width(available_space.width, available_space.width, None, scaling)),
            minimum_height: size_constraint.minimum_height.and_then(|dimension| {
                dimension.try_resolve_height(Some(available_space.height), Some(available_space.height), None, scaling)
            }),
            maximum_height: size_constraint.maximum_height.and_then(|dimension| {
                dimension.try_resolve_height(Some(available_space.height), Some(available_space.height), None, scaling)
            }),
        }
    }

    pub fn derive(
        &self,
        size_constraint: &SizeConstraint,
        available_space: PartialScreenSize,
        unusable_space: ScreenSize,
        scaling: f32,
    ) -> Self {
        Self {
            minimum_width: size_constraint.minimum_width.and_then(|dimension| {
                dimension.try_resolve_width(
                    available_space.width,
                    available_space.width,
                    self.minimum_width.map(|width| (width - unusable_space.width).max(0.0)),
                    scaling,
                )
            }),
            maximum_width: size_constraint.maximum_width.and_then(|dimension| {
                dimension.try_resolve_width(
                    available_space.width,
                    available_space.width,
                    self.maximum_width.map(|width| (width - unusable_space.width).max(0.0)),
                    scaling,
                )
            }),
            minimum_height: size_constraint.minimum_height.and_then(|dimension| {
                dimension.try_resolve_height(
                    available_space.height,
                    available_space.height,
                    self.minimum_height.map(|height| (height - unusable_space.height).max(0.0)),
                    scaling,
                )
            }),
            maximum_height: size_constraint.maximum_height.and_then(|dimension| {
                dimension.try_resolve_height(
                    available_space.height,
                    available_space.height,
                    self.maximum_height.map(|height| (height - unusable_space.height).max(0.0)),
                    scaling,
                )
            }),
        }
    }
}
