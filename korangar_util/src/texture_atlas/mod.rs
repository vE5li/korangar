//! Provides the implementation of online and offline texture atlases.

mod offline;
mod online;

use cgmath::{Point2, Vector2};
pub use offline::{AllocationId, OfflineTextureAtlas};
pub use online::OnlineTextureAtlas;

use crate::Rectangle;

/// Represents an allocated rectangle in the texture atlas.
#[derive(Copy, Clone, Debug)]
pub struct AtlasAllocation {
    /// The rectangle that was allocated.
    pub rectangle: Rectangle<u32>,
    /// The size of the atlas.
    pub atlas_size: Vector2<u32>,
}

impl AtlasAllocation {
    /// Maps normalized input coordinates to normalized atlas coordinates.
    pub fn map_to_atlas(&self, normalized_coordinates: Point2<f32>) -> Point2<f32> {
        // Textured coordinates, that are "clearly bigger" than 1.0 are wrapping. There
        // are some values, even though they are for example "1.0112", which are not
        // wrapped in the original client. So we chose "1.5" as a cutoff point. A value
        // too low could lead to wrongly applied textures.
        let wrapped = normalized_coordinates.map(|value: f32| if value > 1.5 { value.fract() } else { value });
        let x = ((wrapped.x * self.rectangle.width() as f32) + self.rectangle.min.x as f32) / self.atlas_size.x as f32;
        let y = ((wrapped.y * self.rectangle.height() as f32) + self.rectangle.min.y as f32) / self.atlas_size.y as f32;
        Point2::new(x, y)
    }
}
