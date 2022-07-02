use derive_new::new;
use serde::{ Serialize, Deserialize };

use interface::types::{ Dimension, Size, Position, PartialSize };

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

    pub fn resolve(&self, avalible: Size, remaining: Size, scaling: f32) -> PartialSize {

        let width = self.width.resolve_width(avalible.x, remaining.x, scaling);
        let width = self.validated_width(width, avalible.x, remaining.x, scaling);

        let mut height = self.height.resolve_height(avalible.y.into(), remaining.y.into(), scaling);
        if let Some(height) = &mut height {
            *height = self.validated_height(*height, avalible.y.into(), remaining.y.into(), scaling);
        }

        PartialSize::new(width, height)
    }

    pub fn resolve_partial(&self, avalible: PartialSize, remaining: PartialSize, scaling: f32) -> PartialSize {

        let width = self.width.resolve_width(avalible.x, remaining.x, scaling);
        let width = self.validated_width(width, avalible.x, remaining.x, scaling);

        let mut height = self.height.resolve_height(avalible.y, remaining.y, scaling);
        if let Some(height) = &mut height {
            *height = self.validated_height(*height, avalible.y, remaining.y, scaling);
        }

        PartialSize::new(width, height)
    }

    fn validated_width(&self, mut width: f32, avalible: f32, remaining: f32, scaling: f32) -> f32 {

        if let Some(maximum_width) = self.maximum_width {
            let maximum_value = maximum_width.resolve_width(avalible, remaining, scaling);
            width = f32::min(width, maximum_value);
        }

        if let Some(minimum_width) = self.minimum_width {
            let minimum_value = minimum_width.resolve_width(avalible, remaining, scaling);
            width = f32::max(width, minimum_value);
        }

        width
    }

    pub fn validated_height(&self, mut height: f32, avalible: Option<f32>, remaining: Option<f32>, scaling: f32) -> f32 {

        if let Some(maximum_height) = self.maximum_height {
            let maximum_value = maximum_height.resolve_height(avalible, remaining, scaling);
            height = f32::min(height, maximum_value.expect("maximum height cannot be flexible"));
        }

        if let Some(minimum_height) = self.minimum_height {
            let minimum_value = minimum_height.resolve_height(avalible, remaining, scaling);
            height = f32::max(height, minimum_value.expect("minimum height cannot be flexible"));
        }

        height
    }

    pub fn validated_size(&self, size: Size, avalible: Size, scaling: f32) -> Size {
        let width = self.validated_width(size.x, avalible.x, avalible.x, scaling);
        let height = self.validated_height(size.y, avalible.y.into(), avalible.y.into(), scaling);
        Size::new(width, height)
    }

    pub fn validated_position(&self, position: Position, size: Size, avalible: Size) -> Position {
        let half_size = size / 2.0;
        let x = f32::clamp(position.x, -half_size.x, avalible.x - half_size.x);
        let y = f32::clamp(position.y, 0.0, avalible.y - 30.0);
        Position::new(x, y)
    }
}
