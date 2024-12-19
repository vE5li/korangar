use cgmath::Point2;
use hashbrown::HashMap;
use korangar_util::Rectangle;
use serde::Deserialize;

use super::GlyphCoordinate;

#[derive(Debug, Deserialize)]
pub struct FontMapDescriptor {
    pub atlas: Atlas,
    pub metrics: Metrics,
    pub glyphs: Vec<Glyph>,
}

impl FontMapDescriptor {
    pub(crate) fn verify(&self) {
        assert_eq!(self.atlas.atlas_type, AtlasType::Msdf);
        assert_eq!(self.atlas.distance_range, 8);
        assert_eq!(self.atlas.distance_range_middle, 0);
        assert_eq!(self.metrics.em_size, 1.0);
    }
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Atlas {
    pub atlas_type: AtlasType,
    pub distance_range: u32,
    pub distance_range_middle: u32,
    pub size: u32,
    pub width: u32,
    pub height: u32,
    pub y_origin: YOrigin,
}

#[derive(Debug, Deserialize, Ord, PartialOrd, PartialEq, Eq)]
#[allow(unused)]
pub enum AtlasType {
    Msdf,
}

#[derive(Debug, Deserialize)]
pub enum YOrigin {
    Bottom,
    Top,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Metrics {
    pub em_size: f64,
    pub line_height: f64,
    pub ascender: f64,
    pub descender: f64,
    pub underline_y: f64,
    pub underline_thickness: f64,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Glyph {
    pub index: u16,
    pub advance: f64,
    #[serde(default)]
    pub plane_bounds: Option<Bounds>,
    #[serde(default)]
    pub atlas_bounds: Option<Bounds>,
}

#[derive(Debug, Deserialize)]
pub struct Bounds {
    pub left: f64,
    pub bottom: f64,
    pub right: f64,
    pub top: f64,
}

impl FontMapDescriptor {
    pub(crate) fn parse_glyph_cache(&self) -> HashMap<u16, GlyphCoordinate> {
        let mut glyph_map = HashMap::with_capacity(self.glyphs.len());
        let atlas_height = self.atlas.height as f32;
        let atlas_width = self.atlas.width as f32;

        for glyph in &self.glyphs {
            let Some(atlas_bounds) = glyph.atlas_bounds.as_ref() else {
                continue;
            };

            let Some(plane_bounds) = glyph.plane_bounds.as_ref() else {
                continue;
            };

            let texture_coordinate = Rectangle::new(
                Point2::new(atlas_bounds.left as f32 / atlas_width, match self.atlas.y_origin {
                    YOrigin::Bottom => 1.0 - (atlas_bounds.bottom as f32 / atlas_height),
                    YOrigin::Top => atlas_bounds.top as f32 / atlas_height,
                }),
                Point2::new(atlas_bounds.right as f32 / atlas_width, match self.atlas.y_origin {
                    YOrigin::Bottom => 1.0 - (atlas_bounds.top as f32 / atlas_height),
                    YOrigin::Top => atlas_bounds.bottom as f32 / atlas_height,
                }),
            );

            let width = (plane_bounds.right - plane_bounds.left) as f32;
            let height = (plane_bounds.bottom - plane_bounds.top) as f32;

            let offset_left = plane_bounds.left as f32;
            let offset_top = -plane_bounds.top as f32;

            let coordinate = GlyphCoordinate {
                texture_coordinate,
                width,
                height,
                offset_top,
                offset_left,
            };

            glyph_map.insert(glyph.index, coordinate);
        }

        glyph_map
    }
}
