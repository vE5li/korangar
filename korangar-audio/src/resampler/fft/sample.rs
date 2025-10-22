use realfft::FftNum;

/// The trait governing a single sample.
///
/// There are two types which implements this trait so far:
/// * [f32]
/// * [f64]
pub(crate) trait Sample
where
    Self: Copy
        + CoerceFrom<usize>
        + CoerceFrom<f64>
        + CoerceFrom<f32>
        + FftNum
        + std::ops::Mul
        + std::ops::Div
        + std::ops::Add
        + std::ops::Sub
        + std::ops::MulAssign
        + std::ops::RemAssign
        + std::ops::DivAssign
        + std::ops::SubAssign
        + std::ops::AddAssign
        + Send,
{
    const PI: Self;

    /// Calculate the sine of `self`.
    fn sin(self) -> Self;

    /// Coerce `value` into the current type.
    ///
    /// Coercions are governed through the private `CoerceFrom` trait.
    fn coerce<T>(value: T) -> Self
    where
        Self: CoerceFrom<T>,
    {
        Self::coerce_from(value)
    }
}

impl Sample for f32 {
    const PI: Self = std::f32::consts::PI;

    fn sin(self) -> Self {
        f32::sin(self)
    }
}

impl Sample for f64 {
    const PI: Self = std::f64::consts::PI;

    fn sin(self) -> Self {
        f64::sin(self)
    }
}

/// The trait used to coerce a value infallibly from one type to another.
///
/// This is similar to doing `value as T` where `T` is a floating point type.
/// Loss of precision may happen during coercions if the coerced from value
/// doesn't fit fully within the target type.
pub(crate) trait CoerceFrom<T> {
    /// Perform a coercion from `value` into the current type.
    fn coerce_from(value: T) -> Self;
}

impl CoerceFrom<usize> for f32 {
    fn coerce_from(value: usize) -> Self {
        value as f32
    }
}

impl CoerceFrom<usize> for f64 {
    fn coerce_from(value: usize) -> Self {
        value as f64
    }
}

impl CoerceFrom<f64> for f32 {
    fn coerce_from(value: f64) -> Self {
        value as f32
    }
}

impl CoerceFrom<f64> for f64 {
    fn coerce_from(value: f64) -> Self {
        value
    }
}

impl CoerceFrom<f32> for f32 {
    fn coerce_from(value: f32) -> Self {
        value
    }
}

impl CoerceFrom<f32> for f64 {
    fn coerce_from(value: f32) -> Self {
        value as f64
    }
}
