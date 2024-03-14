#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::Dimension;
use crate::application::{PartialSizeTrait, PositionTrait, ScalingTrait, SizeTrait, SizeTraitExt};

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DimensionBound {
    pub size: Dimension,
    pub minimum_size: Option<Dimension>,
    pub maximum_size: Option<Dimension>,
}

impl DimensionBound {
    pub const RELATIVE_ONE_HUNDRED: Self = Self {
        size: Dimension::Relative(100.0),
        minimum_size: None,
        maximum_size: None,
    };

    pub fn add_height(&self, height_bound: DimensionBound) -> SizeBound {
        SizeBound {
            width: self.size,
            minimum_width: self.minimum_size,
            maximum_width: self.maximum_size,
            height: height_bound.size,
            minimum_height: height_bound.minimum_size,
            maximum_height: height_bound.maximum_size,
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SizeBound {
    pub width: Dimension,
    pub minimum_width: Option<Dimension>,
    pub maximum_width: Option<Dimension>,
    pub height: Dimension,
    pub minimum_height: Option<Dimension>,
    pub maximum_height: Option<Dimension>,
}

impl SizeBound {
    pub(crate) const fn only_height(height: Dimension) -> Self {
        Self {
            width: Dimension::Relative(100.0),
            minimum_width: None,
            maximum_width: None,
            height,
            minimum_height: None,
            maximum_height: None,
        }
    }

    pub(crate) fn resolve_window<Size>(&self, available: impl SizeTrait, remaining: impl SizeTrait, scaling: impl ScalingTrait) -> Size
    where
        Size: PartialSizeTrait,
    {
        let parent_limits = ParentLimits::default();

        let width = self.width.resolve_width(available.width(), remaining.width(), None, scaling);
        let width = self.validated_width(width, available.width(), remaining.width(), &parent_limits, scaling);

        let mut height = self
            .height
            .resolve_height(Some(available.height()), Some(remaining.height()), None, scaling);

        if let Some(height) = &mut height {
            *height = self.validated_height(
                *height,
                Some(available.height()),
                Some(remaining.height()),
                &parent_limits,
                scaling,
            );
        }

        Size::new(width, height)
    }

    pub(crate) fn resolve_element<Size>(
        &self,
        available: impl PartialSizeTrait,
        remaining: impl PartialSizeTrait,
        parent_limits: &ParentLimits,
        scaling: impl ScalingTrait,
    ) -> Size
    where
        Size: PartialSizeTrait,
    {
        let width = self.width.resolve_width(available.width(), remaining.width(), None, scaling);
        let width = self.validated_width(width, available.width(), remaining.width(), parent_limits, scaling);

        let mut height = self.height.resolve_height(available.height(), remaining.height(), None, scaling);
        if let Some(height) = &mut height {
            *height = self.validated_height(*height, available.height(), remaining.height(), parent_limits, scaling);
        }

        Size::new(width, height)
    }

    fn validated_width(
        &self,
        mut width: f32,
        available: f32,
        remaining: f32,
        parent_limits: &ParentLimits,
        scaling: impl ScalingTrait,
    ) -> f32 {
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

    pub(crate) fn validated_height(
        &self,
        mut height: f32,
        available: Option<f32>,
        remaining: Option<f32>,
        parent_limits: &ParentLimits,
        scaling: impl ScalingTrait,
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

    pub(crate) fn validated_window_size<Size>(&self, size: impl SizeTrait, available: impl SizeTrait, scaling: impl ScalingTrait) -> Size
    where
        Size: SizeTrait,
    {
        let parent_limits = ParentLimits::default();

        let width = self.validated_width(size.width(), available.width(), available.width(), &parent_limits, scaling);
        let height = self.validated_height(
            size.height(),
            Some(available.height()),
            Some(available.height()),
            &parent_limits,
            scaling,
        );

        Size::new(width, height)
    }

    pub(crate) fn validated_position<Position>(
        &self,
        position: impl PositionTrait,
        size: impl SizeTrait,
        available: impl SizeTrait,
    ) -> Position
    where
        Position: PositionTrait,
    {
        let half_size = size.halved();
        let left = f32::clamp(position.left(), -half_size.width(), available.width() - half_size.width());
        let top = f32::clamp(position.top(), 0.0, available.height() - 30.0);

        Position::new(left, top)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ParentLimits {
    pub minimum_width: Option<f32>,
    pub maximum_width: Option<f32>,
    pub minimum_height: Option<f32>,
    pub maximum_height: Option<f32>,
}

impl ParentLimits {
    pub fn from_bound(size_bound: &SizeBound, available_space: impl SizeTrait, scaling: impl ScalingTrait) -> Self {
        Self {
            minimum_width: size_bound
                .minimum_width
                .and_then(|dimension| dimension.try_resolve_width(available_space.width(), available_space.width(), None, scaling)),
            maximum_width: size_bound
                .maximum_width
                .and_then(|dimension| dimension.try_resolve_width(available_space.width(), available_space.width(), None, scaling)),
            minimum_height: size_bound.minimum_height.and_then(|dimension| {
                dimension.try_resolve_height(Some(available_space.height()), Some(available_space.height()), None, scaling)
            }),
            maximum_height: size_bound.maximum_height.and_then(|dimension| {
                dimension.try_resolve_height(Some(available_space.height()), Some(available_space.height()), None, scaling)
            }),
        }
    }

    pub fn derive(
        &self,
        size_bound: &SizeBound,
        available_space: impl PartialSizeTrait,
        unusable_space: impl SizeTrait,
        scaling: impl ScalingTrait,
    ) -> Self {
        Self {
            minimum_width: size_bound.minimum_width.and_then(|dimension| {
                dimension.try_resolve_width(
                    available_space.width(),
                    available_space.width(),
                    self.minimum_width.map(|width| (width - unusable_space.width()).max(0.0)),
                    scaling,
                )
            }),
            maximum_width: size_bound.maximum_width.and_then(|dimension| {
                dimension.try_resolve_width(
                    available_space.width(),
                    available_space.width(),
                    self.maximum_width.map(|width| (width - unusable_space.width()).max(0.0)),
                    scaling,
                )
            }),
            minimum_height: size_bound.minimum_height.and_then(|dimension| {
                dimension.try_resolve_height(
                    available_space.height(),
                    available_space.height(),
                    self.minimum_height.map(|height| (height - unusable_space.height()).max(0.0)),
                    scaling,
                )
            }),
            maximum_height: size_bound.maximum_height.and_then(|dimension| {
                dimension.try_resolve_height(
                    available_space.height(),
                    available_space.height(),
                    self.maximum_height.map(|height| (height - unusable_space.height()).max(0.0)),
                    scaling,
                )
            }),
        }
    }
}
