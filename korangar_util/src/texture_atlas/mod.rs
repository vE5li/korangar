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
        let x = ((normalized_coordinates.x * self.rectangle.width() as f32) + self.rectangle.min.x as f32) / self.atlas_size.x as f32;
        let y = ((normalized_coordinates.y * self.rectangle.height() as f32) + self.rectangle.min.y as f32) / self.atlas_size.y as f32;
        Point2::new(x, y)
    }
}
