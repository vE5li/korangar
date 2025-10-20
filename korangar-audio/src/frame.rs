use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

/// A stereo audio sample.
#[derive(Copy, Clone, PartialEq, Default)]
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

/// Given a previous frame, a current frame, the two next frames, and a position
/// `x` from 0.0 to 1.0 between the current frame and next frame, get an
/// approximated frame.
///
/// This is the 4-point, 3rd-order Hermite interpolation x-form algorithm from
/// "Polynomial Interpolators for High-Quality Resampling of Oversampled Audio"
/// by Olli Niemitalo, p. 43: http://yehar.com/blog/wp-content/uploads/2009/08/deip.pdf
#[must_use]
pub(crate) fn interpolate_frame(previous: Frame, current: Frame, next_1: Frame, next_2: Frame, fraction: f32) -> Frame {
    let c0 = current;
    let c1 = (next_1 - previous) * 0.5;
    let c2 = previous - current * 2.5 + next_1 * 2.0 - next_2 * 0.5;
    let c3 = (next_2 - previous) * 0.5 + (current - next_1) * 1.5;
    ((c3 * fraction + c2) * fraction + c1) * fraction + c0
}
