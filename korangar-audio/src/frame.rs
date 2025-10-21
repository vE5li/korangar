use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use bytemuck::{Pod, Zeroable};

/// A stereo audio sample.
#[derive(Copy, Clone, PartialEq, Default, Pod, Zeroable)]
#[repr(C)]
pub(crate) struct Frame {
    /// The sample for the left channel.
    pub(crate) left: f32,
    /// The sample for the right channel.
    pub(crate) right: f32,
}

impl Frame {
    /// A [`Frame`] with both the left and right samples set to `0.0`.
    pub(crate) const ZERO: Frame = Frame { left: 0.0, right: 0.0 };

    /// Creates a frame with the given left and right values.
    #[must_use]
    pub(crate) fn new(left: f32, right: f32) -> Self {
        Self { left, right }
    }

    /// Creates a frame with both the left and right channels set to the same
    /// value.
    #[must_use]
    pub(crate) fn from_mono(value: f32) -> Self {
        Self::new(value, value)
    }

    /// Returns the frame mixed down to mono.
    #[must_use]
    pub(crate) fn as_mono(self) -> Self {
        Self::from_mono((self.left + self.right) / 2.0)
    }
}

impl Add for Frame {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.left + rhs.left, self.right + rhs.right)
    }
}

impl AddAssign for Frame {
    fn add_assign(&mut self, rhs: Self) {
        self.left += rhs.left;
        self.right += rhs.right;
    }
}

impl Sub for Frame {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.left - rhs.left, self.right - rhs.right)
    }
}

impl SubAssign for Frame {
    fn sub_assign(&mut self, rhs: Self) {
        self.left -= rhs.left;
        self.right -= rhs.right;
    }
}

impl Mul<f32> for Frame {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.left * rhs, self.right * rhs)
    }
}

impl MulAssign<f32> for Frame {
    fn mul_assign(&mut self, rhs: f32) {
        self.left *= rhs;
        self.right *= rhs;
    }
}

impl Div<f32> for Frame {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self::new(self.left / rhs, self.right / rhs)
    }
}

impl DivAssign<f32> for Frame {
    fn div_assign(&mut self, rhs: f32) {
        self.left /= rhs;
        self.right /= rhs;
    }
}

impl Neg for Frame {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(-self.left, -self.right)
    }
}
