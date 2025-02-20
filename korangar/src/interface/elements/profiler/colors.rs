use std::collections::{BTreeMap, btree_map};
use std::hash::{DefaultHasher, Hash, Hasher};

use crate::graphics::Color;

const GOLDEN_RATIO_CONJUGATE: f64 = 0.618034;

#[derive(Default)]
pub(super) struct ColorLookup {
    colors: BTreeMap<&'static str, Color>,
}

impl ColorLookup {
    pub fn get_color(&mut self, string: &'static str) -> Color {
        *self.colors.entry(string).or_insert_with(|| random_color(string))
    }

    pub fn into_iter(self) -> btree_map::IntoIter<&'static str, Color> {
        self.colors.into_iter()
    }
}

fn random_color(string: &str) -> Color {
    let mut hasher = DefaultHasher::new();
    string.hash(&mut hasher);
    let hash = hasher.finish();

    let hue_base = (hash as f64) / (u64::MAX as f64);
    let saturation_value = ((hash >> 8) & 0xFFFF) as u32;
    let brightness_value = ((hash >> 24) & 0xFFFF) as u32;

    // Rotate by golden ratio conjugate for pleasing distribution
    let hue = (((hue_base + GOLDEN_RATIO_CONJUGATE) % 1.0) * 360.0) as u32;

    // Map saturation to a pleasing range (75-90)
    let saturation = 75 + (saturation_value % 15);

    // Map brightness to a pleasing range (85-100)
    let brightness = 85 + (brightness_value % 15);

    hsb_to_rgb(hue, saturation, brightness)
}

fn hsb_to_rgb(mut hue: u32, saturation: u32, brightness: u32) -> Color {
    if hue == 0 {
        hue = 1
    }

    if hue == 360 {
        hue = 359
    }

    let h: f32 = hue as f32 / 360.0;
    let s: f32 = saturation as f32 / 100.0;
    let b: f32 = brightness as f32 / 100.0;

    let h_i = (h * 6.0).floor();
    let f = h * 6.0 - h_i;
    let p = b * (1.0 - s);
    let q = b * (1.0 - f * s);
    let t = b * (1.0 - (1.0 - f) * s);

    let (r, g, b) = match h_i as i64 {
        0 => (b, t, p),
        1 => (q, b, p),
        2 => (p, b, t),
        3 => (p, q, b),
        4 => (t, p, b),
        _ => (b, p, q),
    };

    Color::rgb(r, g, b)
}
