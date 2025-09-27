use cgmath::{BaseNum, Point2, Vector2};

/// Represents a rectangle in 2D space.
#[derive(Copy, Clone, Debug)]
pub struct Rectangle<N> {
    /// The minimal point of the rectangle (should be top left).
    pub min: Point2<N>,
    /// The maximal point of the rectangle (should be bottom right).
    pub max: Point2<N>,
}

impl<N: BaseNum> Rectangle<N> {
    /// Creates a new [`Rectangle`] with given minimum and maximum coordinates.
    pub fn new(min: Point2<N>, max: Point2<N>) -> Self {
        Self { min, max }
    }

    /// Returns the height of the rectangle.
    pub fn height(&self) -> N {
        self.max.y - self.min.y
    }

    /// Returns the width of the rectangle.
    pub fn width(&self) -> N {
        self.max.x - self.min.x
    }

    /// Checks if this rectangle can fit another rectangle of given size.
    pub fn can_fit(&self, size: Vector2<N>) -> bool {
        self.width() >= size.x && self.height() >= size.y
    }

    /// Tests if the two rectangles overlap.
    pub fn overlaps(&self, other: Rectangle<N>) -> bool {
        self.min.x < other.max.x && self.max.x > other.min.x && self.min.y < other.max.y && self.max.y > other.min.y
    }

    /// Tests if the other rectangle is contained in the current rectangle.
    pub fn contains(&self, other: Rectangle<N>) -> bool {
        self.min.x <= other.min.x && self.min.y <= other.min.y && self.max.x >= other.max.x && self.max.y >= other.max.y
    }
}

impl<N: BaseNum> PartialEq for Rectangle<N> {
    fn eq(&self, other: &Self) -> bool {
        self.min == other.min && self.max == other.max
    }
}

impl<N> From<Rectangle<N>> for [N; 4] {
    fn from(value: Rectangle<N>) -> Self {
        [value.min.x, value.min.y, value.max.x, value.max.y]
    }
}
