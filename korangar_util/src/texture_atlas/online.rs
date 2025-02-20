//! A simple texture atlas for online generation.
use cgmath::{Point2, Vector2};

use super::AtlasAllocation;
use crate::{Rectangle, create_simple_key};

create_simple_key!(NodeId, "A key for a node in the texture atlas");

/// A texture atlas using a skyline packing algorithm.
/// This is optimized for dynamic allocation of rectangular spaces,
/// particularly useful for font glyph rendering.
pub struct OnlineTextureAtlas {
    skyline: Vec<Point2<u32>>,
    width: u32,
    height: u32,
    add_padding: bool,
}

impl OnlineTextureAtlas {
    /// Creates a new online texture atlas with the specified dimensions.
    pub fn new(width: u32, height: u32, add_padding: bool) -> Self {
        assert!(width > 0 && height > 0);

        Self {
            skyline: vec![Point2::new(0, 0)],
            width,
            height,
            add_padding,
        }
    }

    /// Clears the texture atlas.
    pub fn clear(&mut self) {
        self.skyline.clear();
        self.skyline.push(Point2::new(0, 0));
    }

    /// Attempts to allocate space for a rectangle of the given size.
    /// Returns `None` if there's no space left.
    pub fn allocate(&mut self, size: Vector2<u32>) -> Option<AtlasAllocation> {
        if size.x == 0 || size.y == 0 || size.x > self.width || size.y > self.height {
            return None;
        }

        let allocation_size = if self.add_padding {
            Vector2::new(size.x + 2, size.y + 2)
        } else {
            size
        };

        let (best_x, best_y, best_index_start, best_index_end) = self.find_best_position(allocation_size.x, allocation_size.y)?;
        self.update_skyline(
            best_index_start,
            best_index_end,
            best_x,
            best_y,
            allocation_size.x,
            allocation_size.y,
        );

        let rectangle = if self.add_padding {
            Rectangle::new(
                Point2::new(best_x + 1, best_y + 1),
                Point2::new(best_x + size.x + 1, best_y + size.y + 1),
            )
        } else {
            Rectangle::new(Point2::new(best_x, best_y), Point2::new(best_x + size.x, best_y + size.y))
        };

        Some(AtlasAllocation {
            rectangle,
            atlas_size: Vector2::new(self.width, self.height),
        })
    }

    // Implements the SKYLINE-BL heuristic.
    fn find_best_position(&self, rectangle_width: u32, rectangle_height: u32) -> Option<(u32, u32, usize, usize)> {
        let mut best_position = None;
        let mut best_height = u32::MAX;

        for index_start in 0..self.skyline.len() {
            let skyline_x = self.skyline[index_start].x;
            let skyline_y = self.skyline[index_start].y;

            if rectangle_width > self.width - skyline_x {
                break;
            }

            if skyline_y >= best_height {
                continue;
            }

            if let Some((suitable_height, index_end)) = self.find_suitable_height(index_start, skyline_x, rectangle_width, best_height) {
                if suitable_height + rectangle_height <= self.height && suitable_height < best_height {
                    best_height = suitable_height;
                    best_position = Some((skyline_x, suitable_height, index_start, index_end));
                }
            }
        }

        best_position
    }

    fn find_suitable_height(&self, start: usize, x: u32, width: u32, height_limit: u32) -> Option<(u32, usize)> {
        let mut suitable_height = self.skyline[start].y;
        let mut index_end = start + 1;

        while index_end < self.skyline.len() && self.skyline[index_end].x <= x + width {
            suitable_height = suitable_height.max(self.skyline[index_end].y);

            if suitable_height >= height_limit {
                return None;
            }

            index_end += 1;
        }

        Some((suitable_height, index_end))
    }

    fn update_skyline(&mut self, best_index_start: usize, best_index_end: usize, x: u32, y: u32, width: u32, height: u32) {
        let new_top_left = Point2::new(x, y + height);
        let new_bottom_right = Point2::new(
            x + width,
            if best_index_end > 0 {
                self.skyline[best_index_end - 1].y
            } else {
                0
            },
        );

        let bottom_right_point = if best_index_end < self.skyline.len() {
            new_bottom_right.x < self.skyline[best_index_end].x
        } else {
            new_bottom_right.x < self.width
        };

        self.skyline.drain(best_index_start..best_index_end);

        self.skyline.insert(best_index_start, new_top_left);
        if bottom_right_point {
            self.skyline.insert(best_index_start + 1, new_bottom_right);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_allocation() {
        let mut atlas = OnlineTextureAtlas::new(512, 512, false);

        let allocation = atlas.allocate(Vector2::new(100, 100));
        assert!(allocation.is_some());

        let allocation = allocation.unwrap();
        assert_eq!(allocation.rectangle.min, Point2::new(0, 0));
        assert_eq!(allocation.rectangle.max, Point2::new(100, 100));
        assert_eq!(allocation.atlas_size, Vector2::new(512, 512));
    }

    #[test]
    fn test_multiple_allocations() {
        let mut atlas = OnlineTextureAtlas::new(512, 512, false);
        let alloc1 = atlas.allocate(Vector2::new(100, 100)).unwrap();
        let alloc2 = atlas.allocate(Vector2::new(100, 100)).unwrap();

        assert!(!alloc1.rectangle.overlaps(alloc2.rectangle));
    }
}
