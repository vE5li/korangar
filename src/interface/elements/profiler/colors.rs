use std::collections::{btree_map, BTreeMap};

use crate::graphics::Color;

#[derive(Default)]
pub(super) struct ColorLookup {
    colors: BTreeMap<&'static str, Color>,
}

impl ColorLookup {
    pub fn get_color(&mut self, string: &'static str) -> Color {
        *self.colors.entry(string).or_insert_with(|| {
            let [red, green, blue] = random_color::RandomColor::new().seed(string).to_rgb_array();
            Color::rgb(red, green, blue)
        })
    }

    pub fn into_iter(self) -> btree_map::IntoIter<&'static str, Color> {
        self.colors.into_iter()
    }
}
