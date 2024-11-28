//! Commonly used color functionality.

use fast_srgb8::{f32_to_srgb8, srgb8_to_f32};

/// Pre-multiplies the alpha of a sRGB gamma encoded pixel.
pub fn premultiply_alpha(srgba_bytes: &mut [u8]) {
    srgba_bytes.chunks_exact_mut(4).for_each(|chunk| {
        let mut red = srgb8_to_f32(chunk[0]);
        let mut green = srgb8_to_f32(chunk[1]);
        let mut blue = srgb8_to_f32(chunk[2]);
        let alpha = chunk[3] as f32 / 255.0;

        red *= alpha;
        blue *= alpha;
        green *= alpha;

        chunk[0] = f32_to_srgb8(red);
        chunk[1] = f32_to_srgb8(green);
        chunk[2] = f32_to_srgb8(blue);
    });
}
