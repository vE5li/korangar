//! This is a hard fork of rubato, which is licensed under MIT.
//! It simplifies the API, caches FFT configurations that are expensive to
//! calculate and uses a different window function.
//!
//! We choose Kaiser β=10 window function instead of BlackmanHarris2 (squared).
//! BlackmanHarris2 provides 180 dB stopband attenuation but requires 0.26π
//! transition bandwidth. For 16-bit audio (96 dB dynamic range), this is
//! overkill.
//!
//! Kaiser β=10 provides 100 dB stopband attenuation with 0.16π transition
//! bandwidth, preserving more high-frequency content. This is especially
//! important for our 22050→48000 Hz conversions, where BlackmanHarris2 would
//! roll off audible content starting at 9.6 kHz, noticeably darkening the
//! audio.

mod error;
mod sample;
mod sinc;
mod windows;

use std::array;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::{Arc, LazyLock, Mutex};

use error::ResampleError;
use realfft::num_complex::Complex;
use realfft::num_traits::Zero;
use realfft::{ComplexToReal, RealFftPlanner, RealToComplex};

use self::sample::Sample;
use self::sinc::make_sincs_kaiser;
use self::windows::calculate_cutoff_kaiser;

pub(crate) struct FftCacheData<T> {
    filter_f: Arc<[Complex<T>]>,
    fft: Arc<dyn RealToComplex<T>>,
    ifft: Arc<dyn ComplexToReal<T>>,
}

impl<T> Clone for FftCacheData<T> {
    fn clone(&self) -> Self {
        Self {
            filter_f: Arc::clone(&self.filter_f),
            fft: Arc::clone(&self.fft),
            ifft: Arc::clone(&self.ifft),
        }
    }
}

pub(crate) trait FftCache: Sample + Copy {
    fn get_cache() -> &'static Mutex<HashMap<(usize, usize), FftCacheData<Self>>>;
}

impl FftCache for f32 {
    #[allow(clippy::type_complexity)]
    fn get_cache() -> &'static Mutex<HashMap<(usize, usize), FftCacheData<Self>>> {
        static FFT_CACHE_F32: LazyLock<Mutex<HashMap<(usize, usize), FftCacheData<f32>>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
        &FFT_CACHE_F32
    }
}

impl FftCache for f64 {
    #[allow(clippy::type_complexity)]
    fn get_cache() -> &'static Mutex<HashMap<(usize, usize), FftCacheData<Self>>> {
        static FFT_CACHE_F64: LazyLock<Mutex<HashMap<(usize, usize), FftCacheData<f64>>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
        &FFT_CACHE_F64
    }
}

/// A synchronous resampler that uses FFT.
///
/// The resampling is done by FFT:ing the input data. The spectrum is then
/// extended or truncated as well as multiplied with an antialiasing filter
/// before it's inverse transformed to get the resampled waveforms.
pub(crate) struct Fft<const CHANNEL: usize, T> {
    chunk_size_in: usize,
    chunk_size_out: usize,
    fft_size_in: usize,
    fft_size_out: usize,
    overlaps: [Vec<T>; CHANNEL],
    input_scratch: [Vec<T>; CHANNEL],
    output_scratch: [Vec<T>; CHANNEL],
    saved_frames: usize,
    resampler: FftResampler<T>,
}

impl<const CHANNEL: usize, T: FftCache> Fft<CHANNEL, T> {
    /// Create a new `Fft` synchronous resampler.
    ///
    /// The delay from the resampler depends on the length of the FFT.
    ///
    /// Parameters are:
    /// - `sample_rate_input`: Input sample rate.
    /// - `sample_rate_output`: Output sample rate.
    pub(crate) fn new(sample_rate_input: NonZeroUsize, sample_rate_output: NonZeroUsize) -> Self {
        let sample_rate_input = sample_rate_input.get();
        let sample_rate_output = sample_rate_output.get();

        let gcd = greatest_common_divisor(sample_rate_input, sample_rate_output);

        // Use known optimized chunk sizes that result in efficient mixed radix FFTs.
        let chunk_size = match (sample_rate_input, sample_rate_output) {
            (22050, 48000) => 1152,
            (44100, 48000) => 1152,
            (22050, 44100) => 2048,
            (48000, 44100) => 1280,
            _ => 1024,
        };

        let min_chunk_in = sample_rate_input / gcd;
        let fft_chunks = (chunk_size as f32 / min_chunk_in as f32).ceil() as usize;

        let fft_size_out = fft_chunks * sample_rate_output / gcd;
        let fft_size_in = fft_chunks * sample_rate_input / gcd;

        let resampler = FftResampler::<T>::new(fft_size_in, fft_size_out);

        let overlaps: [Vec<T>; CHANNEL] = array::from_fn(|_| vec![T::zero(); fft_size_out]);

        let saved_frames = 0;

        let (chunk_size_in, chunk_size_out) = Self::calc_chunk_sizes(fft_size_in, fft_size_out, chunk_size);

        let needed_input_buffer_size = chunk_size_in + fft_size_in;
        let needed_output_buffer_size = chunk_size_out + fft_size_out;
        let input_scratch: [Vec<T>; CHANNEL] = array::from_fn(|_| vec![T::zero(); needed_input_buffer_size]);
        let output_scratch: [Vec<T>; CHANNEL] = array::from_fn(|_| vec![T::zero(); needed_output_buffer_size]);

        Fft {
            chunk_size_in,
            chunk_size_out,
            fft_size_in,
            fft_size_out,
            overlaps,
            input_scratch,
            output_scratch,
            saved_frames,
            resampler,
        }
    }

    fn calc_chunk_sizes(fft_size_in: usize, fft_size_out: usize, chunk_size: usize) -> (usize, usize) {
        let subchunks_needed = (chunk_size as f32 / fft_size_in as f32).ceil() as usize;
        let frames_needed_in = subchunks_needed * fft_size_in;
        let frames_needed_out = subchunks_needed * fft_size_out;
        (frames_needed_in, frames_needed_out)
    }

    /// Input and output must be interleaved f32 slices. For example stereo
    /// would need to have the format [L0, R0, L1, R1, ...].
    pub(crate) fn process_into_buffer(&mut self, input: &[T], output: &mut [T]) -> Result<(usize, usize), ResampleError> {
        validate_buffers(input, output, CHANNEL, self.chunk_size_in, self.chunk_size_out)?;

        // Deinterleave input into per-channel scratch buffers.
        for frame_idx in 0..self.chunk_size_in {
            for chan in 0..CHANNEL {
                self.input_scratch[chan][frame_idx] = input[frame_idx * CHANNEL + chan];
            }
        }

        let (subchunks_to_process, output_scratch_offset) = (self.chunk_size_in / self.fft_size_in, self.saved_frames);

        // Resample between input and output scratch buffers.
        for chan in 0..CHANNEL {
            for (in_chunk, out_chunk) in self.input_scratch[chan]
                .chunks(self.fft_size_in)
                .take(subchunks_to_process)
                .zip(self.output_scratch[chan][output_scratch_offset..].chunks_mut(self.fft_size_out))
            {
                self.resampler.resample_unit(in_chunk, out_chunk, &mut self.overlaps[chan]);
            }
        }

        // Deinterleave output from per-channel scratch buffers.
        for frame_idx in 0..self.chunk_size_out {
            for chan in 0..CHANNEL {
                output[frame_idx * CHANNEL + chan] = self.output_scratch[chan][frame_idx];
            }
        }

        let input_size = self.chunk_size_in;
        let output_size = self.chunk_size_out;
        Ok((input_size, output_size))
    }

    pub(crate) fn input_frames_max(&self) -> usize {
        self.chunk_size_in
    }

    pub(crate) fn input_frames_next(&self) -> usize {
        self.chunk_size_in
    }

    pub(crate) fn output_frames_max(&self) -> usize {
        self.chunk_size_out
    }

    pub(crate) fn output_frames_next(&self) -> usize {
        self.chunk_size_out
    }
}

fn validate_buffers<T>(
    input: &[T],
    output: &[T],
    channels: usize,
    min_input_frames: usize,
    min_output_frames: usize,
) -> Result<(), ResampleError> {
    let expected_input_len = channels * min_input_frames;
    let min_output_len = channels * min_output_frames;

    if input.len() < expected_input_len {
        return Err(ResampleError::InsufficientInputBufferSize {
            expected: min_input_frames,
            actual: input.len() / channels,
        });
    }

    if output.len() < min_output_len {
        return Err(ResampleError::InsufficientOutputBufferSize {
            expected: min_output_frames,
            actual: output.len() / channels,
        });
    }

    Ok(())
}

/// A helper for resampling a single chunk of data.
struct FftResampler<T> {
    fft_size_in: usize,
    fft_size_out: usize,
    filter_f: Arc<[Complex<T>]>,
    fft: Arc<dyn RealToComplex<T>>,
    ifft: Arc<dyn ComplexToReal<T>>,
    scratch_fw: Vec<Complex<T>>,
    scratch_inv: Vec<Complex<T>>,
    input_buf: Vec<T>,
    input_f: Vec<Complex<T>>,
    output_f: Vec<Complex<T>>,
    output_buf: Vec<T>,
}

impl<T: FftCache> FftResampler<T> {
    pub(crate) fn new(fft_size_in: usize, fft_size_out: usize) -> Self {
        let cached = Self::get_or_create_cached(fft_size_in, fft_size_out);

        let input_f: Vec<Complex<T>> = vec![Complex::zero(); fft_size_in + 1];
        let input_buf: Vec<T> = vec![T::zero(); 2 * fft_size_in];
        let output_f: Vec<Complex<T>> = vec![Complex::zero(); fft_size_out + 1];
        let output_buf: Vec<T> = vec![T::zero(); 2 * fft_size_out];

        let scratch_fw = cached.fft.make_scratch_vec();
        let scratch_inv = cached.ifft.make_scratch_vec();

        FftResampler {
            fft_size_in,
            fft_size_out,
            filter_f: cached.filter_f,
            fft: cached.fft,
            ifft: cached.ifft,
            scratch_fw,
            scratch_inv,
            input_buf,
            input_f,
            output_f,
            output_buf,
        }
    }

    fn get_or_create_cached(fft_size_in: usize, fft_size_out: usize) -> FftCacheData<T> {
        let cache = T::get_cache();

        cache
            .lock()
            .unwrap()
            .entry((fft_size_in, fft_size_out))
            .or_insert_with(|| {
                let cutoff = match fft_size_in > fft_size_out {
                    true => calculate_cutoff_kaiser::<f32>(fft_size_out) * fft_size_out as f32 / fft_size_in as f32,
                    false => calculate_cutoff_kaiser::<f32>(fft_size_in),
                };

                let sinc = make_sincs_kaiser::<T>(fft_size_in, 1, cutoff);
                let mut filter_t: Vec<T> = vec![T::zero(); 2 * fft_size_in];
                let mut filter_f: Vec<Complex<T>> = vec![Complex::zero(); fft_size_in + 1];

                for (n, f) in filter_t.iter_mut().enumerate().take(fft_size_in) {
                    *f = sinc[0][n] / T::coerce(2 * fft_size_in);
                }

                let mut planner = RealFftPlanner::<T>::new();
                let fft = planner.plan_fft_forward(2 * fft_size_in);
                let ifft = planner.plan_fft_inverse(2 * fft_size_out);

                fft.process(&mut filter_t, &mut filter_f).unwrap();

                FftCacheData {
                    filter_f: filter_f.into(),
                    fft,
                    ifft,
                }
            })
            .clone()
    }

    /// Resample a small chunk.
    fn resample_unit(&mut self, wave_in: &[T], wave_out: &mut [T], overlap: &mut [T]) {
        // Copy to input buffer and clear padding area.
        self.input_buf[0..self.fft_size_in].copy_from_slice(wave_in);
        for item in self.input_buf.iter_mut().skip(self.fft_size_in).take(self.fft_size_in) {
            *item = T::zero();
        }

        // FFT and store result in history, update index.
        self.fft
            .process_with_scratch(&mut self.input_buf, &mut self.input_f, &mut self.scratch_fw)
            .unwrap();

        let new_len = match self.fft_size_in < self.fft_size_out {
            true => self.fft_size_in + 1,
            false => self.fft_size_out,
        };

        // Multiply with filter FT.
        self.input_f
            .iter_mut()
            .take(new_len)
            .zip(self.filter_f.iter())
            .for_each(|(spec, filt)| *spec *= filt);

        // Copy to modified spectrum.
        self.output_f[0..new_len].copy_from_slice(&self.input_f[0..new_len]);
        for val in self.output_f[new_len..].iter_mut() {
            *val = Complex::zero();
        }

        // IFFT result, store result and overlap.
        self.ifft
            .process_with_scratch(&mut self.output_f, &mut self.output_buf, &mut self.scratch_inv)
            .unwrap();
        for (n, item) in wave_out.iter_mut().enumerate().take(self.fft_size_out) {
            *item = self.output_buf[n] + overlap[n];
        }
        overlap.copy_from_slice(&self.output_buf[self.fft_size_out..]);
    }
}

fn greatest_common_divisor(a: usize, b: usize) -> usize {
    let mut a = a;
    let mut b = b;

    while b != 0 {
        let temp = b;
        b = a % b;
        a = temp;
    }

    a
}

#[cfg(test)]
mod tests {
    use super::greatest_common_divisor;

    #[test]
    fn test_gcd_basic_cases() {
        assert_eq!(greatest_common_divisor(48, 18), 6);
        assert_eq!(greatest_common_divisor(100, 35), 5);
        assert_eq!(greatest_common_divisor(54, 24), 6);
    }
}
