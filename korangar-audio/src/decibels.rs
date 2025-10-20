use crate::tween::Tweenable;

#[derive(Clone, Copy, PartialEq, PartialOrd)]
/// Represents a change in volume.
///
/// Higher values increase the volume and lower values decrease it.
/// Setting the volume of a sound to -60dB or lower makes it silent.
pub(crate) struct Decibels(pub(crate) f32);

impl Decibels {
    /// The decibel value that produces no change in volume.
    pub(crate) const IDENTITY: Self = Self(0.0);
    /// The minimum decibel value at which a sound is considered
    /// silent.
    pub(crate) const SILENCE: Self = Self(-60.0);

    /// Converts decibels to amplitude, a linear volume measurement.
    ///
    /// This returns a number from `0.0`-`1.0` that you can multiply
    /// a singal by to change its volume.
    pub(crate) fn as_amplitude(self) -> f32 {
        // Adding a special case for db == 0.0 improves performance in the sound
        // playback benchmarks by about 7%
        if self == Self(0.0) {
            return 1.0;
        }
        if self <= Self::SILENCE {
            return 0.0;
        }
        10.0f32.powf(self.0 / 20.0)
    }
}

impl Default for Decibels {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Tweenable for Decibels {
    fn interpolate(a: Self, b: Self, amount: f64) -> Self {
        Self(Tweenable::interpolate(a.0, b.0, amount))
    }
}

impl From<f32> for Decibels {
    fn from(value: f32) -> Self {
        Self(value)
    }
}
