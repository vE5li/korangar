//! Smooth interpolation between values.

use cgmath::{Point3, Quaternion};

/// A trait for types that can be smoothly interpolated.
pub(crate) trait Tweenable: Copy {
    /// Returns a linearly interpolated value between `a` and `b`.
    ///
    /// An amount of `0.0` should yield `a`, an amount of `1.0` should
    /// yield `b`, and an amount of `0.5` should yield a value halfway
    /// between `a` and `b`.
    #[must_use]
    fn interpolate(a: Self, b: Self, amount: f64) -> Self;
}

impl Tweenable for f32 {
    fn interpolate(a: Self, b: Self, amount: f64) -> Self {
        a + (b - a) * amount as f32
    }
}

impl Tweenable for f64 {
    fn interpolate(a: Self, b: Self, amount: f64) -> Self {
        a + (b - a) * amount
    }
}

impl Tweenable for Point3<f32> {
    fn interpolate(a: Self, b: Self, amount: f64) -> Self {
        a + (b - a) * amount as f32
    }
}

impl Tweenable for Quaternion<f32> {
    fn interpolate(a: Self, b: Self, amount: f64) -> Self {
        a.slerp(b, amount as f32)
    }
}
