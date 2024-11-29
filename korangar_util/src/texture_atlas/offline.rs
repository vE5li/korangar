//! A simple texture atlas for deferred offline generation.

use std::num::NonZeroU32;

use cgmath::{EuclideanSpace, Point2, Vector2};
use image::{imageops, Rgba, RgbaImage};

use super::AtlasAllocation;
use crate::container::{SecondarySimpleSlab, SimpleSlab};
use crate::{create_simple_key, Rectangle};

/// Factor we used to increase the texture size for inefficiency in
/// the packing algorithm.
const EFFICIENCY_FACTOR: f32 = 1.05;

create_simple_key!(AllocationId, "A key for an allocation");

/// A texture atlas implementation using the MAXRECTS-BSSF (Best Short Side Fit)
/// algorithm.
///
/// This implementation is based on the algorithm described in the paper:
/// "A Thousand Ways to Pack the Bin - A Practical Approach to Two-Dimensional
/// Rectangle Bin Packing" by Jukka Jylänki (2010).
///
/// Key features of this implementation:
/// - Pre-sorts the input using Descending Short Side Sort (DESCSS) for better
///   packing efficient.
/// - Uses the Best Short Side Fit (BSSF) heuristic for rectangle placement,
///   which has been shown to produce very efficient packings.
///
/// Performance characteristics:
/// - Excellent packing efficiency in both online and offline scenarios.
/// - In our deferred, offline mode, MAXRECTS-BSSF-DESCSS's performance has been
///   shown to be excellent.
/// - Theoretical worst-case time complexity is O(n³), but practical performance
///   is much better.
///
/// This implementation is particularly effective when the input can be sorted.
pub struct OfflineTextureAtlas {
    size: Vector2<u32>,
    free_rects: Vec<Rectangle<u32>>,
    deferred_allocation: SimpleSlab<AllocationId, DeferredAllocation>,
    allocations: SecondarySimpleSlab<AllocationId, AtlasAllocation>,
    image: Option<RgbaImage>,
    add_padding: bool,
    mip_level_count: u32,
    padding: u32,
}

struct DeferredAllocation {
    image: RgbaImage,
    original_size: Vector2<u32>,
    padded_size: Vector2<u32>,
}

impl OfflineTextureAtlas {
    /// Creates a new texture atlas. If `mip_level_count` is defined, the
    /// padding and the atlas texture itself is properly chosen to be compatible
    /// for the given `mip_level_count`.
    pub fn new(add_padding: bool, mip_level_count: Option<NonZeroU32>) -> Self {
        let mip_level_count = mip_level_count.map(|count| count.get()).unwrap_or(1);
        let padding = if add_padding {
            if mip_level_count > 1 {
                2 << (mip_level_count - 1)
            } else {
                2
            }
        } else {
            0
        };

        OfflineTextureAtlas {
            size: Vector2::new(0, 0),
            free_rects: Vec::default(),
            deferred_allocation: SimpleSlab::default(),
            allocations: SecondarySimpleSlab::default(),
            image: None,
            add_padding,
            mip_level_count,
            padding,
        }
    }

    /// Registers the given image and returns an ID which can be used to get an
    /// allocation after optimization.
    pub fn register_image(&mut self, image: RgbaImage) -> AllocationId {
        if self.image.is_some() {
            panic!("can't register new images once atlas has been build");
        }

        let (x, y) = image.dimensions();
        let original_size = Vector2::new(x, y);
        let padded_size = self.calculate_size(original_size);

        self.deferred_allocation
            .insert(DeferredAllocation {
                image,
                padded_size,
                original_size,
            })
            .expect("deferred allocation slab is full")
    }

    fn calculate_size(&self, mut size: Vector2<u32>) -> Vector2<u32> {
        if self.mip_level_count > 1 {
            let alignment = 1 << (self.mip_level_count - 1);

            if self.padding > 0 {
                size.x += self.padding * 2;
                size.y += self.padding * 2;
            }

            size.x = ((size.x + alignment - 1) / alignment) * alignment;
            size.y = ((size.y + alignment - 1) / alignment) * alignment;
        } else if self.padding > 0 {
            size.x += self.padding * 2;
            size.y += self.padding * 2;
        }

        size
    }

    /// Returns the allocation for the given allocation ID, once data was
    /// inserted and the atlas was generated.
    pub fn get_allocation(&self, allocation_id: AllocationId) -> Option<AtlasAllocation> {
        self.allocations.get(allocation_id).copied()
    }

    /// Builds the atlas with the optimal atlas size.
    pub fn build_atlas(&mut self) {
        if self.image.is_some() {
            panic!("atlas is already build");
        }

        let mut deferred_allocations: Vec<(AllocationId, DeferredAllocation)> = self.deferred_allocation.drain().collect();

        // DESCSS (Descending Short Side Sort)
        deferred_allocations.sort_unstable_by(|a, b| {
            let a_short_side = a.1.padded_size.x.min(a.1.padded_size.y);
            let b_short_side = b.1.padded_size.x.min(b.1.padded_size.y);
            let a_long_side = a.1.padded_size.x.max(a.1.padded_size.y);
            let b_long_side = b.1.padded_size.x.max(b.1.padded_size.y);

            b_short_side.cmp(&a_short_side).then_with(|| b_long_side.cmp(&a_long_side))
        });

        let (mut width, mut height) = self.estimate_initial_size(&deferred_allocations);
        let mut temp_allocations = Vec::new();

        let mut success = false;
        while !success {
            self.size = Vector2::new(width, height);
            self.free_rects = vec![Rectangle::new(Point2::new(0, 0), Point2::from_vec(self.size))];
            success = true;

            temp_allocations.clear();

            for (allocation_id, alloc) in deferred_allocations.iter() {
                if let Some(allocation) = self.allocate(alloc.padded_size, alloc.original_size) {
                    temp_allocations.push((*allocation_id, allocation));
                } else {
                    success = false;

                    if self.mip_level_count > 1 {
                        if width <= height {
                            width *= 2
                        } else {
                            height *= 2
                        }
                    } else {
                        let current_area = width * height;
                        let adjusted_area = (current_area as f32 * EFFICIENCY_FACTOR) as u32;
                        let side = (adjusted_area as f32).sqrt() as u32;
                        width = side;
                        height = side;
                    }

                    break;
                }
            }
        }

        self.image = Some(RgbaImage::from_pixel(width, height, Rgba([255, 0, 255, 255])));
        for (id, allocation) in temp_allocations {
            let (_, deferred_allocation) = deferred_allocations.iter().find(|(alloc_id, _)| *alloc_id == id).unwrap();
            self.write_image_data(&allocation, &deferred_allocation.image);
            self.allocations.insert(id, allocation);
        }
    }

    /// Implements the BSSF (Best Short Side Fit) heuristics.
    fn find_best_rectangle(&self, size: Vector2<u32>) -> Option<usize> {
        self.free_rects
            .iter()
            .enumerate()
            .filter(|(_, rectangle)| rectangle.can_fit(size))
            .min_by_key(|(_, rectangle)| {
                let leftover_horizontal = rectangle.width().saturating_sub(size.x);
                let leftover_vertical = rectangle.height().saturating_sub(size.y);
                std::cmp::min(leftover_horizontal, leftover_vertical)
            })
            .map(|(index, _)| index)
    }

    fn estimate_initial_size(&self, deferred_allocations: &[(AllocationId, DeferredAllocation)]) -> (u32, u32) {
        let total_area: u32 = deferred_allocations.iter().map(|r| r.1.padded_size.x * r.1.padded_size.y).sum();

        if self.mip_level_count > 1 {
            let mut width = 128;
            let mut height = 128;
            let mut expand_width = true;

            while (width * height) < total_area {
                if expand_width {
                    width *= 2;
                    expand_width = false;
                } else {
                    height *= 2;
                    expand_width = true;
                }
            }

            (width, height)
        } else {
            let adjusted_area = (total_area as f32 * EFFICIENCY_FACTOR) as u32;
            let side = (adjusted_area as f32).sqrt() as u32;
            (side, side)
        }
    }

    fn allocate(&mut self, padded_size: Vector2<u32>, original_size: Vector2<u32>) -> Option<AtlasAllocation> {
        let best_rect_index = self.find_best_rectangle(padded_size)?;
        let mut free_rect = self.free_rects.remove(best_rect_index);

        if self.mip_level_count > 1 {
            let alignment = 1 << (self.mip_level_count - 1);
            free_rect.min.x = ((free_rect.min.x + self.padding + alignment - 1) & !(alignment - 1)).saturating_sub(self.padding);
            free_rect.min.y = ((free_rect.min.y + self.padding + alignment - 1) & !(alignment - 1)).saturating_sub(self.padding);
        }

        let used_rect = Rectangle::new(free_rect.min, free_rect.min + padded_size);

        let image_position = if self.add_padding {
            Point2::new(used_rect.min.x + self.padding, used_rect.min.y + self.padding)
        } else {
            used_rect.min
        };

        let allocation = AtlasAllocation {
            rectangle: Rectangle::new(image_position, image_position + original_size),
            atlas_size: self.size,
        };

        let (f_prime, f_double_prime) = self.maxrects_split(free_rect, used_rect);
        self.free_rects.extend([f_prime, f_double_prime].into_iter().flatten());

        self.update_free_rectangles(used_rect);
        self.remove_contained_rectangles();

        Some(allocation)
    }

    /// The actual MAXRECTS splitting as described in the paper.
    fn maxrects_split(&self, free_rect: Rectangle<u32>, used_rect: Rectangle<u32>) -> (Option<Rectangle<u32>>, Option<Rectangle<u32>>) {
        let f_prime =
            (free_rect.max.x > used_rect.max.x).then_some(Rectangle::new(Point2::new(used_rect.max.x, free_rect.min.y), free_rect.max));

        let f_double_prime =
            (free_rect.max.y > used_rect.max.y).then_some(Rectangle::new(Point2::new(free_rect.min.x, used_rect.max.y), free_rect.max));

        (f_prime, f_double_prime)
    }

    /// After allocating a rectangle (used_rect), we need to update our list of
    /// free rectangles to reflect the new state of available space.
    fn update_free_rectangles(&mut self, used_rect: Rectangle<u32>) {
        let mut i = 0;

        while i < self.free_rects.len() {
            if self.free_rects[i].overlaps(used_rect) {
                let free_rect = self.free_rects.swap_remove(i);
                let new_rects = subdivide_rectangle(free_rect, used_rect);
                self.free_rects.extend(new_rects);
            } else {
                i += 1;
            }
        }
    }

    /// Removes rectangles that are fully contained within other rectangles in
    /// the `free_rects` list.
    fn remove_contained_rectangles(&mut self) {
        let mut i = 0;

        while i < self.free_rects.len() {
            let mut contained = false;
            let mut j = 0;

            while j < self.free_rects.len() {
                if i != j && self.free_rects[j].contains(self.free_rects[i]) {
                    contained = true;
                    break;
                }
                j += 1;
            }

            if contained {
                self.free_rects.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }

    /// Saves the atlas image at the given path.
    pub fn save_atlas(&self, path: &str) -> Result<(), image::ImageError> {
        self.image.as_ref().unwrap().save(path)
    }

    /// Returns the bytes of the underlying image buffer.
    pub fn get_atlas(mut self) -> RgbaImage {
        self.image.take().expect("the atlas has not been build yet")
    }

    fn write_image_data(&mut self, allocation: &AtlasAllocation, image: &RgbaImage) {
        let atlas_image = self.image.as_mut().unwrap();

        imageops::replace(
            atlas_image,
            image,
            allocation.rectangle.min.x as _,
            allocation.rectangle.min.y as _,
        );

        if self.padding > 0 {
            let width = allocation.rectangle.width();
            let height = allocation.rectangle.height();

            for x in 0..width {
                let color = image.get_pixel(x, 0);
                for dy in 1..=self.padding {
                    atlas_image.put_pixel(
                        allocation.rectangle.min.x + x,
                        allocation.rectangle.min.y.saturating_sub(dy),
                        *color,
                    );
                }
            }

            for x in 0..width {
                let color = image.get_pixel(x, height - 1);
                for dy in 0..self.padding {
                    atlas_image.put_pixel(allocation.rectangle.min.x + x, allocation.rectangle.max.y + dy, *color);
                }
            }

            for y in 0..height {
                let color = image.get_pixel(0, y);
                for dx in 1..=self.padding {
                    atlas_image.put_pixel(
                        allocation.rectangle.min.x.saturating_sub(dx),
                        allocation.rectangle.min.y + y,
                        *color,
                    );
                }
            }

            for y in 0..height {
                let color = image.get_pixel(width - 1, y);
                for dx in 0..self.padding {
                    atlas_image.put_pixel(allocation.rectangle.max.x + dx, allocation.rectangle.min.y + y, *color);
                }
            }

            let top_left = image.get_pixel(0, 0);
            let top_right = image.get_pixel(width - 1, 0);
            let bottom_left = image.get_pixel(0, height - 1);
            let bottom_right = image.get_pixel(width - 1, height - 1);

            for dy in 1..=self.padding {
                for dx in 1..=self.padding {
                    atlas_image.put_pixel(
                        allocation.rectangle.min.x.saturating_sub(dx),
                        allocation.rectangle.min.y.saturating_sub(dy),
                        *top_left,
                    );
                }
            }

            for dy in 1..=self.padding {
                for dx in 0..self.padding {
                    atlas_image.put_pixel(
                        allocation.rectangle.max.x + dx,
                        allocation.rectangle.min.y.saturating_sub(dy),
                        *top_right,
                    );
                }
            }

            for dy in 0..self.padding {
                for dx in 1..=self.padding {
                    atlas_image.put_pixel(
                        allocation.rectangle.min.x.saturating_sub(dx),
                        allocation.rectangle.max.y + dy,
                        *bottom_left,
                    );
                }
            }

            for dy in 0..self.padding {
                for dx in 0..self.padding {
                    atlas_image.put_pixel(allocation.rectangle.max.x + dx, allocation.rectangle.max.y + dy, *bottom_right);
                }
            }
        }
    }
}

fn subdivide_rectangle(free_rect: Rectangle<u32>, used_rect: Rectangle<u32>) -> Vec<Rectangle<u32>> {
    let mut result = Vec::new();

    // Rectangle on the right side of used_rect.
    if free_rect.max.x > used_rect.max.x {
        result.push(Rectangle::new(Point2::new(used_rect.max.x, free_rect.min.y), free_rect.max));
    }

    // Rectangle below used_rect.
    if free_rect.max.y > used_rect.max.y {
        result.push(Rectangle::new(
            Point2::new(free_rect.min.x, used_rect.max.y),
            Point2::new(free_rect.max.x, free_rect.max.y),
        ));
    }

    // Rectangle on the left side of used_rect.
    if free_rect.min.x < used_rect.min.x {
        result.push(Rectangle::new(free_rect.min, Point2::new(used_rect.min.x, free_rect.max.y)));
    }

    // Rectangle above used_rect.
    if free_rect.min.y < used_rect.min.y {
        result.push(Rectangle::new(free_rect.min, Point2::new(free_rect.max.x, used_rect.min.y)));
    }

    result
}

#[cfg(test)]
mod tests {
    use image::{Rgba, RgbaImage};

    use super::*;

    #[test]
    fn test_allocate_single_rectangle() {
        let mut atlas = OfflineTextureAtlas::new(false, NonZeroU32::new(1));

        let image = RgbaImage::new(100, 100);
        let id = atlas.register_image(image);
        atlas.build_atlas();

        let allocation = atlas.get_allocation(id);
        assert!(allocation.is_some());

        let alloc = allocation.unwrap();
        assert_eq!(alloc.rectangle.width(), 100);
        assert_eq!(alloc.rectangle.height(), 100);
    }

    #[test]
    fn test_multiple_allocations() {
        let mut atlas = OfflineTextureAtlas::new(false, NonZeroU32::new(1));
        let id1 = atlas.register_image(RgbaImage::new(100, 100));
        let id2 = atlas.register_image(RgbaImage::new(200, 200));
        let id3 = atlas.register_image(RgbaImage::new(300, 300));
        atlas.build_atlas();

        let allocation1 = atlas.get_allocation(id1).unwrap();
        let allocation2 = atlas.get_allocation(id2).unwrap();
        let allocation3 = atlas.get_allocation(id3).unwrap();

        assert_eq!(allocation1.rectangle.width(), 100);
        assert_eq!(allocation1.rectangle.height(), 100);
        assert_eq!(allocation2.rectangle.width(), 200);
        assert_eq!(allocation2.rectangle.height(), 200);
        assert_eq!(allocation3.rectangle.width(), 300);
        assert_eq!(allocation3.rectangle.height(), 300);
    }

    #[test]
    fn test_no_rectangle_overlap() {
        let mut atlas = OfflineTextureAtlas::new(false, NonZeroU32::new(1));
        let mut ids = Vec::new();

        for _ in 0..10 {
            ids.push(atlas.register_image(RgbaImage::new(100, 100)));
        }

        atlas.build_atlas();

        let allocations: Vec<_> = ids.iter().map(|&id| atlas.get_allocation(id).unwrap()).collect();

        for (i, alloc1) in allocations.iter().enumerate() {
            for (j, alloc2) in allocations.iter().enumerate() {
                if i != j {
                    assert!(
                        !alloc1.rectangle.overlaps(alloc2.rectangle),
                        "Overlap detected between rectangle {} and rectangle {}",
                        i,
                        j
                    );
                }
            }
        }
    }

    #[test]
    fn test_no_rectangle_overlap_varied_sizes() {
        let mut atlas = OfflineTextureAtlas::new(false, NonZeroU32::new(1));
        let sizes = [(50, 50), (200, 200), (100, 100), (300, 100), (100, 300), (25, 25), (400, 400)];

        let mut ids = Vec::new();
        for &(width, height) in sizes.iter() {
            ids.push(atlas.register_image(RgbaImage::new(width, height)));
        }

        for _ in 0..20 {
            ids.push(atlas.register_image(RgbaImage::new(10, 10)));
        }

        atlas.build_atlas();

        let allocations: Vec<_> = ids.iter().map(|&id| atlas.get_allocation(id).unwrap()).collect();

        for (i, alloc1) in allocations.iter().enumerate() {
            for (j, alloc2) in allocations.iter().enumerate() {
                if i != j {
                    assert!(
                        !alloc1.rectangle.overlaps(alloc2.rectangle),
                        "Overlap detected between rectangle {} ({:?}) and rectangle {} ({:?})",
                        i,
                        alloc1.rectangle,
                        j,
                        alloc2.rectangle
                    );
                }
            }
        }
    }

    #[test]
    fn test_atlas_with_padding() {
        let mut atlas = OfflineTextureAtlas::new(true, NonZeroU32::new(1));
        let image = RgbaImage::from_pixel(10, 10, Rgba([255, 0, 0, 255]));
        let id = atlas.register_image(image);
        atlas.build_atlas();
        let allocation = atlas.get_allocation(id).unwrap();

        assert_eq!(allocation.rectangle.width(), 10);
        assert_eq!(allocation.rectangle.height(), 10);

        let atlas_image = atlas.get_atlas();

        let top_padding = atlas_image.get_pixel(allocation.rectangle.min.x, allocation.rectangle.min.y - 1);
        let bottom_padding = atlas_image.get_pixel(allocation.rectangle.min.x, allocation.rectangle.max.y);
        let left_padding = atlas_image.get_pixel(allocation.rectangle.min.x - 1, allocation.rectangle.min.y);
        let right_padding = atlas_image.get_pixel(allocation.rectangle.max.x, allocation.rectangle.min.y);

        assert_eq!(*top_padding, Rgba([255, 0, 0, 255]));
        assert_eq!(*bottom_padding, Rgba([255, 0, 0, 255]));
        assert_eq!(*left_padding, Rgba([255, 0, 0, 255]));
        assert_eq!(*right_padding, Rgba([255, 0, 0, 255]));
    }
}
