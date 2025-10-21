use super::sample::Sample;
use crate::resampler::fft::windows::make_window_kaiser;

/// Helper function: sinc(x) = sin(pi*x)/(pi*x).
pub(crate) fn sinc<T: Sample>(value: T) -> T {
    match value == T::zero() {
        true => T::one(),
        false => (value * T::PI).sin() / (value * T::PI),
    }
}

/// Helper function. Make a set of windowed sincs.
pub(crate) fn make_sincs_kaiser<T: Sample>(npoints: usize, factor: usize, f_cutoff: f32) -> Vec<Vec<T>> {
    let totpoints = npoints * factor;
    let mut y = Vec::with_capacity(totpoints);
    let window = make_window_kaiser::<T>(totpoints);
    let mut sum = T::zero();
    for (x, w) in window.iter().enumerate().take(totpoints) {
        let val = *w * sinc((T::coerce(x) - T::coerce(totpoints / 2)) * T::coerce(f_cutoff) / T::coerce(factor));
        sum += val;
        y.push(val);
    }
    sum /= T::coerce(factor);

    let mut sincs = vec![vec![T::zero(); npoints]; factor];
    for p in 0..npoints {
        for n in 0..factor {
            sincs[factor - n - 1][p] = y[factor * p + n] / sum;
        }
    }

    sincs
}
