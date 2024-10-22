//! A simple texture atlas for deferred offline generation.

use cgmath::Vector2;
use image::{imageops, RgbaImage};

use crate::container::{SecondarySimpleSlab, SimpleSlab};
use crate::create_simple_key;

/// Factor we used to increase the texture size for inefficiency in
/// the packing algorithm.
const EFFICIENCY_FACTOR: f32 = 1.05;

/// Represents a rectangle in 2D space.
#[derive(Copy, Clone, Debug)]
pub struct Rectangle {
    /// The minimal point of the rectangle (should be top left).
    pub min: Vector2<u32>,
    /// The maximal point of the rectangle (should be bottom right).
    pub max: Vector2<u32>,
}

impl Rectangle {
    /// Creates a new [`Rectangle`] with given minimum and maximum coordinates.
    pub fn new(min: Vector2<u32>, max: Vector2<u32>) -> Self {
        Self { min, max }
    }

    /// Returns the height of the rectangle.
    pub fn height(&self) -> u32 {
        self.max.y - self.min.y
    }

    /// Returns the width of the rectangle.
    pub fn width(&self) -> u32 {
        self.max.x - self.min.x
    }

    /// Checks if this rectangle can fit another rectangle of given size.
    fn can_fit(&self, size: Vector2<u32>) -> bool {
        self.width() >= size.x && self.height() >= size.y
    }

    /// Tests if the two rectangles overlap.
    fn overlaps(&self, other: Rectangle) -> bool {
        self.min.x < other.max.x && self.max.x > other.min.x && self.min.y < other.max.y && self.max.y > other.min.y
    }

    /// Tests if the other rectangle is contained in the current rectangle.
    fn contains(&self, other: Rectangle) -> bool {
        self.min.x <= other.min.x && self.min.y <= other.min.y && self.max.x >= other.max.x && self.max.y >= other.max.y
    }
}

impl PartialEq for Rectangle {
    fn eq(&self, other: &Self) -> bool {
        self.min == other.min && self.max == other.max
    }
}

create_simple_key!(AllocationId, "A key for an allocation");

/// Represents an allocated rectangle in the texture atlas.
#[derive(Copy, Clone, Debug)]
pub struct AtlasAllocation {
    /// The rectangle that was allocated.
    pub rectangle: Rectangle,
    /// The final size of the atlas.
    atlas_size: Vector2<u32>,
}

impl AtlasAllocation {
    /// Maps normalized input coordinates to normalized atlas coordinates.
    pub fn map_to_atlas(&self, normalized_coordinates: Vector2<f32>) -> Vector2<f32> {
        let x = ((normalized_coordinates.x * self.rectangle.width() as f32) + self.rectangle.min.x as f32) / self.atlas_size.x as f32;
        let y = ((normalized_coordinates.y * self.rectangle.height() as f32) + self.rectangle.min.y as f32) / self.atlas_size.y as f32;
        Vector2::new(x, y)
    }
}

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
pub struct TextureAtlas {
    size: Vector2<u32>,
    free_rects: Vec<Rectangle>,
    deferred_allocation: SimpleSlab<AllocationId, DeferredAllocation>,
    allocations: SecondarySimpleSlab<AllocationId, AtlasAllocation>,
    image: Option<RgbaImage>,
    add_padding: bool,
}

struct DeferredAllocation {
    image: RgbaImage,
    size: Vector2<u32>,
}

impl TextureAtlas {
    /// Creates a new texture atlas.
    pub fn new(add_padding: bool) -> Self {
        TextureAtlas {
            size: Vector2::new(0, 0),
            free_rects: Vec::default(),
            deferred_allocation: SimpleSlab::default(),
            allocations: SecondarySimpleSlab::default(),
            image: None,
            add_padding,
        }
    }

    /// Registers the given image and returns an ID which can be used to get an
    /// allocation after optimization.
    pub fn register_image(&mut self, image: RgbaImage) -> AllocationId {
        if self.image.is_some() {
            panic!("can't register new images once atlas has been build");
        }

        let (x, y) = image.dimensions();

        let size = if self.add_padding {
            // We add two pixel at each side for texture not bleeding into each other.
            // We copy one pixel later from the edges of the image into the padding, because
            // how GPU sampling works. The other pixel ist not filled and is mainly done,
            // because in some empty textures there could occur bleed in extreme angles
            // otherwise.
            Vector2::new(x + 4, y + 4)
        } else {
            Vector2::new(x, y)
        };

        self.deferred_allocation
            .insert(DeferredAllocation { image, size })
            .expect("deferred allocation slab is full")
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
            let a_short_side = a.1.size.x.min(a.1.size.y);
            let b_short_side = b.1.size.x.min(b.1.size.y);
            let a_long_side = a.1.size.x.max(a.1.size.y);
            let b_long_side = b.1.size.x.max(b.1.size.y);

            b_short_side.cmp(&a_short_side).then_with(|| b_long_side.cmp(&a_long_side))
        });

        let (mut width, mut height) = self.estimate_initial_size(&deferred_allocations);
        let mut temp_allocations = Vec::new();

        let mut success = false;
        while !success {
            self.size = Vector2::new(width, height);
            self.free_rects = vec![Rectangle::new(Vector2::new(0, 0), self.size)];
            success = true;

            temp_allocations.clear();

            for (allocation_id, alloc) in deferred_allocations.iter() {
                if let Some(allocation) = self.allocate(alloc.size) {
                    temp_allocations.push((*allocation_id, allocation));
                } else {
                    success = false;
                    let current_area = width * height;
                    let adjusted_area = (current_area as f32 * EFFICIENCY_FACTOR) as u32;
                    let side = (adjusted_area as f32).sqrt() as u32;
                    width = side;
                    height = side;
                    break;
                }
            }
        }

        self.image = Some(RgbaImage::new(width, height));
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
        let total_area: u32 = deferred_allocations.iter().map(|r| r.1.size.x * r.1.size.y).sum();
        let adjusted_area = (total_area as f32 * EFFICIENCY_FACTOR) as u32;
        let side = (adjusted_area as f32).sqrt() as u32;
        (side, side)
    }

    fn allocate(&mut self, size: Vector2<u32>) -> Option<AtlasAllocation> {
        let best_rect_index = self.find_best_rectangle(size)?;
        let free_rect = self.free_rects.remove(best_rect_index);

        let allocation = if self.add_padding {
            AtlasAllocation {
                rectangle: Rectangle::new(
                    Vector2::new(free_rect.min.x + 2, free_rect.min.y + 2),
                    Vector2::new(free_rect.min.x + size.x - 2, free_rect.min.y + size.y - 2),
                ),
                atlas_size: self.size,
            }
        } else {
            AtlasAllocation {
                rectangle: Rectangle::new(free_rect.min, free_rect.min + size),
                atlas_size: self.size,
            }
        };

        let used_rect = Rectangle::new(free_rect.min, free_rect.min + size);
        let (f_prime, f_double_prime) = self.maxrects_split(free_rect, used_rect);
        self.free_rects.extend([f_prime, f_double_prime].into_iter().flatten());

        self.update_free_rectangles(used_rect);
        self.remove_contained_rectangles();

        Some(allocation)
    }

    /// The actual MAXRECTS splitting as described in the paper.
    fn maxrects_split(&self, free_rect: Rectangle, used_rect: Rectangle) -> (Option<Rectangle>, Option<Rectangle>) {
        let f_prime =
            (free_rect.max.x > used_rect.max.x).then_some(Rectangle::new(Vector2::new(used_rect.max.x, free_rect.min.y), free_rect.max));

        let f_double_prime =
            (free_rect.max.y > used_rect.max.y).then_some(Rectangle::new(Vector2::new(free_rect.min.x, used_rect.max.y), free_rect.max));

        (f_prime, f_double_prime)
    }

    /// After allocating a rectangle (used_rect), we need to update our list of
    /// free rectangles to reflect the new state of available space.
    fn update_free_rectangles(&mut self, used_rect: Rectangle) {
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

        if self.add_padding {
            let width = allocation.rectangle.width();
            let height = allocation.rectangle.height();

            // Top padding
            for x in 0..width {
                let color = image.get_pixel(x, 0);
                atlas_image.put_pixel(allocation.rectangle.min.x + x, allocation.rectangle.min.y - 1, *color);
            }

            // Bottom padding
            for x in 0..width {
                let color = image.get_pixel(x, height - 1);
                atlas_image.put_pixel(allocation.rectangle.min.x + x, allocation.rectangle.max.y, *color);
            }

            // Left padding
            for y in 0..height {
                let color = image.get_pixel(0, y);
                atlas_image.put_pixel(allocation.rectangle.min.x - 1, allocation.rectangle.min.y + y, *color);
            }

            // Right padding
            for y in 0..height {
                let color = image.get_pixel(width - 1, y);
                atlas_image.put_pixel(allocation.rectangle.max.x, allocation.rectangle.min.y + y, *color);
            }

            // Corner padding
            let top_left = image.get_pixel(0, 0);
            let top_right = image.get_pixel(width - 1, 0);
            let bottom_left = image.get_pixel(0, height - 1);
            let bottom_right = image.get_pixel(width - 1, height - 1);

            atlas_image.put_pixel(allocation.rectangle.min.x - 1, allocation.rectangle.min.y - 1, *top_left);
            atlas_image.put_pixel(allocation.rectangle.max.x, allocation.rectangle.min.y - 1, *top_right);
            atlas_image.put_pixel(allocation.rectangle.min.x - 1, allocation.rectangle.max.y, *bottom_left);
            atlas_image.put_pixel(allocation.rectangle.max.x, allocation.rectangle.max.y, *bottom_right);
        }
    }
}

fn subdivide_rectangle(free_rect: Rectangle, used_rect: Rectangle) -> Vec<Rectangle> {
    let mut result = Vec::new();

    // Rectangle on the right side of used_rect.
    if free_rect.max.x > used_rect.max.x {
        result.push(Rectangle::new(Vector2::new(used_rect.max.x, free_rect.min.y), free_rect.max));
    }

    // Rectangle below used_rect.
    if free_rect.max.y > used_rect.max.y {
        result.push(Rectangle::new(
            Vector2::new(free_rect.min.x, used_rect.max.y),
            Vector2::new(free_rect.max.x, free_rect.max.y),
        ));
    }

    // Rectangle on the left side of used_rect.
    if free_rect.min.x < used_rect.min.x {
        result.push(Rectangle::new(free_rect.min, Vector2::new(used_rect.min.x, free_rect.max.y)));
    }

    // Rectangle above used_rect.
    if free_rect.min.y < used_rect.min.y {
        result.push(Rectangle::new(free_rect.min, Vector2::new(free_rect.max.x, used_rect.min.y)));
    }

    result
}

#[cfg(test)]
mod tests {
    use image::{Rgba, RgbaImage};

    use super::*;

    #[test]
    fn test_allocate_single_rectangle() {
        let mut atlas = TextureAtlas::new(false);

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
        let mut atlas = TextureAtlas::new(false);
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
        let mut atlas = TextureAtlas::new(false);
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
        let mut atlas = TextureAtlas::new(false);
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
        let mut atlas = TextureAtlas::new(true);
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
